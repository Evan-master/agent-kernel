//! Owner-aware resource creation.
//!
//! This module belongs to `agent-kernel-core`. It creates resources with an
//! owning agent and an initial capability in one atomic fixed-capacity
//! operation while preserving bootstrap-only unowned resource registration.

use crate::{
    AgentId, Capability, CapabilityId, Event, EventKind, KernelCore, KernelError, Operation,
    OperationSet, Resource, ResourceCreateOutcome, ResourceId, ResourceKind, ResourceStatus,
    VerificationRequirement,
};

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
    >
{
    pub fn create_resource(
        &mut self,
        agent: AgentId,
        kind: ResourceKind,
        parent: Option<(ResourceId, CapabilityId)>,
        operations: OperationSet,
    ) -> Result<ResourceCreateOutcome, KernelError> {
        self.ensure_agent_active(agent)?;
        let parent_id = if let Some((parent_id, parent_capability)) = parent {
            self.ensure_authorized(agent, parent_capability, parent_id, Operation::Act)?;
            Some(parent_id)
        } else {
            None
        };
        if self.resource_len >= RESOURCES {
            return Err(KernelError::ResourceStoreFull);
        }
        let capability_slot = self
            .capabilities
            .iter()
            .position(|capability| capability.is_none())
            .ok_or(KernelError::CapabilityStoreFull)?;
        self.ensure_event_slots(2)?;

        let resource = ResourceId::new(self.next_resource);
        self.next_resource += 1;
        self.resources[self.resource_len] = Resource {
            id: resource,
            kind,
            parent: parent_id,
            owner: Some(agent),
            status: ResourceStatus::Active,
        };
        self.resource_len += 1;

        let capability = CapabilityId::new(self.next_capability);
        self.next_capability += 1;
        self.capabilities[capability_slot] = Some(Capability {
            id: capability,
            agent,
            resource,
            operations,
            revoked: false,
            task: None,
            parent: None,
        });

        self.record_resource_created_event(agent, resource, capability, operations)?;
        self.record_capability_event(
            EventKind::CapabilityGranted,
            agent,
            resource,
            capability,
            None,
            operations,
            None,
            None,
            None,
        )?;
        Ok(ResourceCreateOutcome {
            resource,
            capability,
        })
    }

    fn record_resource_created_event(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        capability: CapabilityId,
        operations: OperationSet,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::ResourceCreated,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: None,
            operations,
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            signal: None,
            target_agent: None,
            driver_binding: None,
            device_event: None,
            device_event_kind: None,
            device_event_payload: None,
            driver_command: None,
            driver_command_kind: None,
            driver_command_payload: None,
            driver_command_result: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
