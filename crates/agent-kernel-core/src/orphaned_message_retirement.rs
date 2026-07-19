//! Capability-authorized retirement of pending mail for retired Agents.
//!
//! This no_std core module closes orphaned Mailbox records after managed Agent
//! retirement. It validates the actor, management relationship, Delegate
//! authority, pending state, Namespace liveness, and Event capacity before one
//! dense Message Store removal. Message IDs remain monotonic.

use crate::{
    AgentId, AgentStatus, CapabilityId, Event, EventKind, KernelCore, KernelError, MessageId,
    MessageRecord, MessageStatus, NamespaceObject, Operation, OrphanedMessageRetirement,
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
    pub fn retire_orphaned_message(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        message: MessageId,
    ) -> Result<OrphanedMessageRetirement, KernelError> {
        self.ensure_agent_active(actor)?;
        let index = self
            .messages()
            .iter()
            .position(|record| record.id == message)
            .ok_or(KernelError::MessageNotFound)?;
        let record = self.messages[index];
        if record.status != MessageStatus::Pending {
            return Err(KernelError::MessageStatusMismatch);
        }

        let recipient = self.find_agent(record.recipient)?;
        if recipient.status != AgentStatus::Retired {
            return Err(KernelError::OrphanedMessageRetirementNotReady);
        }
        let Some(management_resource) = recipient.management_resource else {
            return Err(KernelError::AgentManagementDenied);
        };
        if recipient.manager.is_none() {
            return Err(KernelError::AgentManagementDenied);
        }
        self.ensure_authorized(actor, authority, management_resource, Operation::Delegate)?;

        if self
            .namespace_entries()
            .iter()
            .any(|entry| entry.object == NamespaceObject::Message(message))
        {
            return Err(KernelError::MessageRetirementReferenced);
        }
        self.ensure_event_slots(1)?;

        self.remove_message_at(index);
        self.record(orphaned_message_retirement_event(record, actor, authority))?;
        Ok(OrphanedMessageRetirement::new(
            record,
            actor,
            authority,
            management_resource,
        ))
    }
}

fn orphaned_message_retirement_event(
    record: MessageRecord,
    actor: AgentId,
    authority: CapabilityId,
) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::OrphanedMessageRetired;
    event.resource = record.payload.resource;
    event.capability = record.payload.capability;
    event.source_capability = Some(authority);
    event.intent = record.payload.intent;
    event.action = record.payload.action;
    event.message = Some(record.id);
    event.message_kind = Some(record.kind);
    event.operation = Some(Operation::Delegate);
    event.task = record.payload.task;
    event.fault = record.payload.fault;
    event.target_agent = Some(record.recipient);
    event
}
