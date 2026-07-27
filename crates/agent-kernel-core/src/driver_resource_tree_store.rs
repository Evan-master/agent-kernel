//! Atomic Driver Resource Tree creation.
//!
//! This core-layer transaction preflights authority, descriptor ranges, store
//! capacity, and event capacity before creating one root Resource plus bounded
//! endpoint-region children. Failure before commit changes no kernel state.

use crate::{
    AgentId, Capability, CapabilityId, DriverEndpointRecord, DriverResourceRegion,
    DriverResourceTree, DriverResourceTreeSpec, EventKind, KernelCore, KernelError, Operation,
    OperationSet, Resource, ResourceCreateOutcome, ResourceId, ResourceKind, ResourceStatus,
    DRIVER_RESOURCE_REGION_CAPACITY,
};

const CAPABILITY_SLOT_CAPACITY: usize = DRIVER_RESOURCE_REGION_CAPACITY + 1;

impl<
        const AGENTS: usize,
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
        const MESSAGES: usize,
        const MEMORY_CELLS: usize,
        const NAMESPACE_ENTRIES: usize,
        const FAULTS: usize,
        const FAULT_HANDLERS: usize,
        const FAULT_POLICIES: usize,
        const WAITERS: usize,
        const AGENT_IMAGES: usize,
        const DRIVER_BINDINGS: usize,
        const DEVICE_EVENTS: usize,
        const DRIVER_COMMANDS: usize,
        const DRIVER_INVOCATIONS: usize,
        const RUNTIME_ADMISSIONS: usize,
    >
    KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        MESSAGES,
        MEMORY_CELLS,
        NAMESPACE_ENTRIES,
        FAULTS,
        FAULT_HANDLERS,
        FAULT_POLICIES,
        WAITERS,
        AGENT_IMAGES,
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
        RUNTIME_ADMISSIONS,
    >
{
    pub fn create_driver_resource_tree(
        &mut self,
        owner: AgentId,
        parent: Option<(ResourceId, CapabilityId)>,
        operations: OperationSet,
        spec: DriverResourceTreeSpec,
    ) -> Result<DriverResourceTree, KernelError> {
        let parent = self.preflight_driver_resource_tree(owner, parent, operations, spec)?;
        let capability_slots = self.driver_tree_capability_slots(spec.region_count() + 1)?;
        Ok(self.commit_driver_resource_tree(owner, parent, operations, spec, capability_slots))
    }

    fn preflight_driver_resource_tree(
        &self,
        owner: AgentId,
        parent: Option<(ResourceId, CapabilityId)>,
        operations: OperationSet,
        spec: DriverResourceTreeSpec,
    ) -> Result<Option<ResourceId>, KernelError> {
        self.ensure_agent_active(owner)?;
        let parent = if let Some((resource, capability)) = parent {
            self.ensure_authorized(owner, capability, resource, Operation::Act)?;
            Some(resource)
        } else {
            None
        };
        if !operations.allows(Operation::Delegate) {
            return Err(KernelError::OperationDenied);
        }
        match spec.root_kind() {
            ResourceKind::Device | ResourceKind::Network | ResourceKind::Service => {}
            _ => return Err(KernelError::ResourceKindMismatch),
        }
        let region_count = spec.region_count();
        if region_count == 0 {
            return Err(KernelError::DriverResourceTreeEmpty);
        }
        if RESOURCES.saturating_sub(self.resource_len) < region_count + 1 {
            return Err(KernelError::ResourceStoreFull);
        }
        if RESOURCES.saturating_sub(self.driver_endpoint_len) < region_count {
            return Err(KernelError::DriverEndpointStoreFull);
        }
        self.driver_tree_capability_slots(region_count + 1)?;
        self.ensure_event_slots(2 + region_count * 3)?;
        self.validate_driver_tree_descriptors(spec)?;
        Ok(parent)
    }

    fn driver_tree_capability_slots(
        &self,
        needed: usize,
    ) -> Result<[usize; CAPABILITY_SLOT_CAPACITY], KernelError> {
        let mut slots = [usize::MAX; CAPABILITY_SLOT_CAPACITY];
        let mut found = 0;
        for (index, capability) in self.capabilities.iter().enumerate() {
            if capability.is_some() {
                continue;
            }
            slots[found] = index;
            found += 1;
            if found == needed {
                return Ok(slots);
            }
        }
        Err(KernelError::CapabilityStoreFull)
    }

    fn validate_driver_tree_descriptors(
        &self,
        spec: DriverResourceTreeSpec,
    ) -> Result<(), KernelError> {
        for (slot, descriptor) in spec.regions().iter().enumerate() {
            let Some(descriptor) = descriptor else {
                continue;
            };
            self.validate_driver_endpoint_descriptor(*descriptor)?;
            self.ensure_driver_endpoint_does_not_overlap(*descriptor)?;
            for previous in spec.regions()[..slot].iter().flatten() {
                if Self::driver_endpoint_descriptors_overlap(*previous, *descriptor)? {
                    return Err(KernelError::DriverEndpointOverlap);
                }
            }
        }
        Ok(())
    }

    fn commit_driver_resource_tree(
        &mut self,
        owner: AgentId,
        parent: Option<ResourceId>,
        operations: OperationSet,
        spec: DriverResourceTreeSpec,
        capability_slots: [usize; CAPABILITY_SLOT_CAPACITY],
    ) -> DriverResourceTree {
        let root = self.append_driver_tree_resource(
            owner,
            spec.root_kind(),
            parent,
            operations,
            capability_slots[0],
        );
        let mut regions = [None; DRIVER_RESOURCE_REGION_CAPACITY];
        let mut capability_slot = 1;
        for (slot, descriptor) in spec.regions().iter().enumerate() {
            let Some(descriptor) = descriptor else {
                continue;
            };
            let created = self.append_driver_tree_resource(
                owner,
                ResourceKind::Device,
                Some(root.resource),
                operations,
                capability_slots[capability_slot],
            );
            capability_slot += 1;
            self.driver_endpoints[self.driver_endpoint_len] = DriverEndpointRecord {
                resource: created.resource,
                installer: owner,
                descriptor: *descriptor,
            };
            self.driver_endpoint_len += 1;
            self.record_driver_endpoint_registered(owner, created.capability, created.resource)
                .expect("Driver Resource Tree event capacity was preflighted");
            regions[slot] = Some(DriverResourceRegion::new(
                slot as u8,
                created.resource,
                created.capability,
                *descriptor,
            ));
        }
        DriverResourceTree::new(root, regions)
    }

    fn append_driver_tree_resource(
        &mut self,
        owner: AgentId,
        kind: ResourceKind,
        parent: Option<ResourceId>,
        operations: OperationSet,
        capability_slot: usize,
    ) -> ResourceCreateOutcome {
        let resource = ResourceId::new(self.next_resource);
        self.next_resource += 1;
        self.resources[self.resource_len] = Resource {
            id: resource,
            kind,
            parent,
            owner: Some(owner),
            status: ResourceStatus::Active,
        };
        self.resource_len += 1;

        let capability = CapabilityId::new(self.next_capability);
        self.next_capability += 1;
        self.capabilities[capability_slot] = Some(Capability {
            id: capability,
            agent: owner,
            resource,
            operations,
            revoked: false,
            task: None,
            parent: None,
        });
        self.record_resource_created_event(owner, resource, capability, operations)
            .expect("Driver Resource Tree event capacity was preflighted");
        self.record_capability_event(
            EventKind::CapabilityGranted,
            owner,
            resource,
            capability,
            None,
            operations,
            None,
            None,
            None,
        )
        .expect("Driver Resource Tree event capacity was preflighted");
        ResourceCreateOutcome {
            resource,
            capability,
        }
    }
}
