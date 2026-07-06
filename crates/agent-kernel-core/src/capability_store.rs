//! Fixed-capacity capability grants and revocation.
//!
//! This module belongs to `agent-kernel-core`. It owns deterministic capability
//! allocation and revocation while preserving the invariant that all grants
//! point at a registered agent and an existing resource.

use crate::{
    AgentId, Capability, CapabilityId, Event, EventKind, IntentId, KernelCore, KernelError,
    OperationSet, ResourceId, TaskId, VerificationRequirement,
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
    >
{
    pub fn grant_capability(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        operations: OperationSet,
    ) -> Result<CapabilityId, KernelError> {
        self.ensure_agent_active(agent)?;
        self.find_resource(resource)?;

        let slot = self
            .capabilities
            .iter()
            .position(|capability| capability.is_none())
            .ok_or(KernelError::CapabilityStoreFull)?;
        self.ensure_event_slots(1)?;

        let id = CapabilityId::new(self.next_capability);
        self.next_capability += 1;
        self.capabilities[slot] = Some(Capability {
            id,
            agent,
            resource,
            operations,
            revoked: false,
            task: None,
            parent: None,
        });
        self.record_capability_event(
            EventKind::CapabilityGranted,
            agent,
            resource,
            id,
            None,
            operations,
            None,
            None,
            None,
        )?;
        Ok(id)
    }

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

        let slot = self
            .capabilities
            .iter()
            .position(|capability| capability.is_none())
            .ok_or(KernelError::CapabilityStoreFull)?;
        self.ensure_event_slots(1)?;

        let id = CapabilityId::new(self.next_capability);
        self.next_capability += 1;
        self.capabilities[slot] = Some(Capability {
            id,
            agent,
            resource,
            operations,
            revoked: false,
            task: Some(task),
            parent: Some(parent),
        });
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

    pub fn revoke_capability(&mut self, capability: CapabilityId) -> Result<(), KernelError> {
        let cap = self.find_capability(capability)?;
        self.ensure_event_slots(1)?;

        self.find_capability_mut(capability)?.revoked = true;
        self.record_capability_event(
            EventKind::CapabilityRevoked,
            cap.agent,
            cap.resource,
            capability,
            None,
            cap.operations,
            cap.task,
            None,
            None,
        )?;
        Ok(())
    }

    fn record_capability_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        resource: ResourceId,
        capability: CapabilityId,
        source_capability: Option<CapabilityId>,
        operations: OperationSet,
        task: Option<TaskId>,
        intent: Option<IntentId>,
        target_agent: Option<AgentId>,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind,
            resource: Some(resource),
            capability: Some(capability),
            source_capability,
            intent,
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
            task,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            target_agent,
        })
    }
}
