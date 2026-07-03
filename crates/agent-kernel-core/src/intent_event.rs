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
    const fn event_kind(self) -> EventKind {
        match self {
            Self::Bound => EventKind::IntentBound,
            Self::Fulfilled => EventKind::IntentFulfilled,
            Self::Cancelled => EventKind::IntentCancelled,
        }
    }
}

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > KernelCore<RESOURCES, CAPS, EVENTS, INTENTS, TASKS, RUN_QUEUE>
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
            kind: kind.event_kind(),
            resource: Some(intent_record.resource),
            capability: None,
            source_capability: None,
            intent: Some(intent_record.id),
            intent_kind: Some(intent_record.kind),
            action: None,
            operation: Some(intent_record.kind.required_operation()),
            operations: OperationSet::empty(),
            verification: intent_record.verification,
            checkpoint: None,
            task: Some(task),
            target_agent: None,
        })
    }
}
