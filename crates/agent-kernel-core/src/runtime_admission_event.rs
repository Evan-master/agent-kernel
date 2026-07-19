//! Replayable event construction for runtime admission transitions.

use crate::{AgentId, CapabilityId, Event, EventKind, Operation, RuntimeAdmissionRecord, Task};

pub(crate) fn runtime_admission_event(record: RuntimeAdmissionRecord, kind: EventKind) -> Event {
    let mut event = Event::empty();
    event.agent = record.requester;
    event.kind = kind;
    event.resource = Some(record.resource);
    event.capability = Some(record.authority);
    event.operation = Some(Operation::Delegate);
    event.task = Some(record.task);
    event.target_agent = Some(record.target);
    event.agent_image = Some(record.image);
    event.runtime_admission = Some(record.id);
    event
}

pub(crate) fn runtime_admission_queue_event(record: RuntimeAdmissionRecord, task: Task) -> Event {
    let mut event = Event::empty();
    event.agent = record.target;
    event.kind = EventKind::TaskQueued;
    event.resource = Some(record.resource);
    event.intent = Some(task.intent);
    event.task = Some(record.task);
    event.target_agent = Some(record.target);
    event.agent_image = Some(record.image);
    event.runtime_admission = Some(record.id);
    event
}

pub(crate) fn runtime_admission_compaction_event(
    record: RuntimeAdmissionRecord,
    actor: AgentId,
    authority: CapabilityId,
) -> Event {
    let mut event = runtime_admission_event(record, EventKind::RuntimeAdmissionCompacted);
    event.agent = actor;
    event.capability = Some(authority);
    event
}
