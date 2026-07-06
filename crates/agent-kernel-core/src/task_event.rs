//! Task-scoped event recording helpers.
//!
//! This module belongs to `agent-kernel-core`. It keeps lifecycle event
//! construction for tasks in one place so task state transitions can stay
//! focused on authorization, status checks, and mutation order.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, OperationSet, TaskId,
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
    >
{
    pub(crate) fn record_task_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: Option<CapabilityId>,
        task: TaskId,
        target_agent: Option<AgentId>,
    ) -> Result<Event, KernelError> {
        let task_record = self.find_task(task)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind,
            resource: Some(task_record.resource),
            capability,
            source_capability: None,
            intent: Some(task_record.intent),
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
            task: Some(task),
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            target_agent,
        })
    }
}
