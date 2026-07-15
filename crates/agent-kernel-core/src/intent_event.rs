//! Task-driven intent lifecycle event helpers.
//!
//! This module belongs to `agent-kernel-core`. It records deterministic
//! task-driven intent lifecycle events using existing task and intent records,
//! without allocating or depending on host runtime behavior.

use crate::{AgentId, Event, EventKind, KernelCore, KernelError, OperationSet, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum IntentTaskEventKind {
    Bound,
    Fulfilled,
    Cancelled,
}

impl IntentTaskEventKind {
    const fn as_event(self) -> EventKind {
        match self {
            Self::Bound => EventKind::IntentBound,
            Self::Fulfilled => EventKind::IntentFulfilled,
            Self::Cancelled => EventKind::IntentCancelled,
        }
    }
}

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
    >
{
    pub(crate) fn record_intent_task_event(
        &mut self,
        kind: IntentTaskEventKind,
        agent: AgentId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        let task_record = self.find_task(task)?;
        let intent_record = self.find_intent(task_record.intent)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind: kind.as_event(),
            resource: Some(intent_record.resource),
            capability: None,
            source_capability: None,
            intent: Some(intent_record.id),
            intent_kind: Some(intent_record.kind),
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(intent_record.kind.required_operation()),
            operations: OperationSet::empty(),
            verification: intent_record.verification,
            checkpoint: None,
            task: Some(task),
            task_result: None,
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
            driver_invocation: None,
            driver_invocation_ticks: None,
            driver_invocation_quantum: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
