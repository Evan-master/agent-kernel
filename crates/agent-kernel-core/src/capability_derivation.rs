//! Capability derivation and attenuation.
//!
//! This module belongs to `agent-kernel-core`. It owns derived capability
//! allocation for task-scoped delegation and general agent-to-agent authority
//! attenuation while preserving fixed-capacity, no_std behavior.

use crate::{
    AgentId, Capability, CapabilityId, EventKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceId, TaskId,
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
    pub(crate) fn derive_task_capability(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        operations: OperationSet,
        task: TaskId,
        parent: CapabilityId,
    ) -> Result<CapabilityId, KernelError> {
        self.ensure_agent_active(agent)?;
        self.find_resource(resource)?;
        let parent_capability = self.find_capability(parent)?;
        let task_record = self.find_task(task)?;

        let slot = self.reserve_capability_slot()?;
        self.ensure_event_slots(1)?;

        let id =
            self.insert_derived_capability(slot, agent, resource, operations, Some(task), parent);
        self.record_capability_event(
            EventKind::CapabilityDerived,
            parent_capability.agent,
            resource,
            id,
            Some(parent),
            operations,
            Some(task),
            Some(task_record.intent),
            Some(agent),
        )?;
        Ok(id)
    }

    pub fn derive_capability(
        &mut self,
        actor: AgentId,
        source_capability: CapabilityId,
        target_agent: AgentId,
        operations: OperationSet,
    ) -> Result<CapabilityId, KernelError> {
        self.ensure_agent_active(actor)?;
        self.ensure_agent_active(target_agent)?;
        let source = self.find_capability(source_capability)?;
        self.find_resource(source.resource)?;
        self.ensure_capability_chain_active(source)?;
        if source.agent != actor {
            return Err(KernelError::AgentMismatch);
        }
        if source.task.is_some() {
            return Err(KernelError::CapabilityScopeMismatch);
        }
        if !source.operations.allows(Operation::Delegate) {
            return Err(KernelError::OperationDenied);
        }
        if !operations.is_subset_of(source.operations) {
            return Err(KernelError::OperationDenied);
        }

        let slot = self.reserve_capability_slot()?;
        self.ensure_event_slots(1)?;

        let id = self.insert_derived_capability(
            slot,
            target_agent,
            source.resource,
            operations,
            None,
            source_capability,
        );
        self.record_capability_event(
            EventKind::CapabilityDerived,
            actor,
            source.resource,
            id,
            Some(source_capability),
            operations,
            None,
            None,
            Some(target_agent),
        )?;
        Ok(id)
    }

    fn reserve_capability_slot(&self) -> Result<usize, KernelError> {
        self.capabilities
            .iter()
            .position(|capability| capability.is_none())
            .ok_or(KernelError::CapabilityStoreFull)
    }

    fn insert_derived_capability(
        &mut self,
        slot: usize,
        agent: AgentId,
        resource: ResourceId,
        operations: OperationSet,
        task: Option<TaskId>,
        parent: CapabilityId,
    ) -> CapabilityId {
        let id = CapabilityId::new(self.next_capability);
        self.next_capability += 1;
        self.capabilities[slot] = Some(Capability {
            id,
            agent,
            resource,
            operations,
            revoked: false,
            task,
            parent: Some(parent),
        });
        id
    }
}
