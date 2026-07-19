//! Recipient-owned retirement of acknowledged mailbox records.
//!
//! This no_std core module validates Agent identity, terminal delivery state,
//! Namespace liveness, and Event capacity before removing one record from the
//! dense fixed-capacity Message Store. Message IDs remain monotonic.

use crate::{
    AgentId, Event, EventKind, KernelCore, KernelError, MessageId, MessageRecord,
    MessageRetirement, MessageStatus, NamespaceObject,
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
    pub fn retire_message(
        &mut self,
        agent: AgentId,
        message: MessageId,
    ) -> Result<MessageRetirement, KernelError> {
        self.ensure_agent_active(agent)?;
        let index = self
            .messages()
            .iter()
            .position(|record| record.id == message)
            .ok_or(KernelError::MessageNotFound)?;
        let record = self.messages[index];
        if record.recipient != agent {
            return Err(KernelError::MessageAgentMismatch);
        }
        if record.status != MessageStatus::Acknowledged {
            return Err(KernelError::MessageStatusMismatch);
        }
        if self
            .namespace_entries()
            .iter()
            .any(|entry| entry.object == NamespaceObject::Message(message))
        {
            return Err(KernelError::MessageRetirementReferenced);
        }
        self.ensure_event_slots(1)?;

        let previous = self.messages;
        let remaining = self.message_len - 1;
        self.messages[index..remaining].copy_from_slice(&previous[index + 1..self.message_len]);
        self.messages[remaining] = MessageRecord::empty();
        self.message_len = remaining;
        self.record(message_retirement_event(record, agent))?;

        Ok(MessageRetirement::new(record))
    }
}

fn message_retirement_event(record: MessageRecord, agent: AgentId) -> Event {
    let mut event = Event::empty();
    event.agent = agent;
    event.kind = EventKind::MessageRetired;
    event.resource = record.payload.resource;
    event.capability = record.payload.capability;
    event.intent = record.payload.intent;
    event.action = record.payload.action;
    event.message = Some(record.id);
    event.message_kind = Some(record.kind);
    event.task = record.payload.task;
    event.fault = record.payload.fault;
    event.target_agent = Some(record.sender);
    event
}
