//! Fixed-capacity resource lifecycle.
//!
//! This module belongs to `agent-kernel-core`. It owns deterministic resource
//! allocation and retirement for the no_std core store. It performs no host I/O
//! and validates parent resources before adding child resources.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, Operation, OperationSet,
    Resource, ResourceId, ResourceKind, ResourceStatus, VerificationRequirement,
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
    >
{
    pub fn register_resource(
        &mut self,
        kind: ResourceKind,
        parent: Option<ResourceId>,
    ) -> Result<ResourceId, KernelError> {
        if let Some(parent_id) = parent {
            self.find_resource(parent_id)?;
        }

        if self.resource_len >= RESOURCES {
            return Err(KernelError::ResourceStoreFull);
        }

        let id = ResourceId::new(self.next_resource);
        self.next_resource += 1;
        self.resources[self.resource_len] = Resource {
            id,
            kind,
            parent,
            owner: None,
            status: ResourceStatus::Active,
        };
        self.resource_len += 1;
        Ok(id)
    }

    pub fn retire_resource(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        self.ensure_authorized(agent, capability, resource, Operation::Rollback)?;
        self.ensure_event_slots(1)?;

        self.find_resource_mut(resource)?.status = ResourceStatus::Retired;
        self.record_resource_event(EventKind::ResourceRetired, agent, capability, resource)
    }

    pub fn resources(&self) -> &[Resource] {
        &self.resources[..self.resource_len]
    }

    fn record_resource_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind,
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
            operations: OperationSet::empty(),
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
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
