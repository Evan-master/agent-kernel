//! Fixed-capacity native mailbox IPC store.
//!
//! This module owns deterministic agent-to-agent message delivery for
//! `agent-kernel-core`. It keeps messages in a fixed array, requires active
//! registered agents at every boundary, preserves FIFO receive order by store
//! position, and records replayable events for every successful mutation.

use crate::{
    AgentId, Event, EventKind, KernelCore, KernelError, MessageId, MessageKind, MessagePayload,
    MessageRecord, MessageStatus,
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
    pub fn send_message(
        &mut self,
        sender: AgentId,
        recipient: AgentId,
        kind: MessageKind,
        payload: MessagePayload,
    ) -> Result<MessageId, KernelError> {
        self.ensure_agent_active(sender)?;
        self.ensure_agent_active(recipient)?;
        self.ensure_message_capacity()?;

        let waiter_index = self.find_mailbox_waiter_index(recipient);
        if let Some(waiter_index) = waiter_index {
            let waiter = self.waiters[waiter_index];
            self.ensure_agent_admitted_for_task(waiter.agent, waiter.task)?;
            self.ensure_run_queue_capacity()?;
            self.ensure_event_slots(2)?;

            let message = self.append_message(sender, recipient, kind, payload);
            self.record_message_event(EventKind::MessageSent, sender, recipient, message)?;
            self.wake_mailbox_waiter(waiter_index, sender, message)?;
            return Ok(message);
        }

        self.ensure_event_slots(1)?;

        let message = self.append_message(sender, recipient, kind, payload);
        self.record_message_event(EventKind::MessageSent, sender, recipient, message)?;
        Ok(message)
    }

    pub fn receive_message(&mut self, agent: AgentId) -> Result<MessageId, KernelError> {
        self.ensure_agent_active(agent)?;
        let index = self
            .oldest_pending_message_index(agent)
            .ok_or(KernelError::MailboxEmpty)?;
        let message = self.messages[index];
        self.ensure_event_slots(1)?;

        self.messages[index].status = MessageStatus::Received;
        self.record_message_event(
            EventKind::MessageReceived,
            agent,
            message.sender,
            message.id,
        )?;
        Ok(message.id)
    }

    pub fn acknowledge_message(
        &mut self,
        agent: AgentId,
        message: MessageId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let record = self.find_message(message)?;
        if record.recipient != agent {
            return Err(KernelError::MessageAgentMismatch);
        }
        if record.status != MessageStatus::Received {
            return Err(KernelError::MessageStatusMismatch);
        }
        self.ensure_event_slots(1)?;

        self.find_message_mut(message)?.status = MessageStatus::Acknowledged;
        self.record_message_event(
            EventKind::MessageAcknowledged,
            agent,
            record.sender,
            message,
        )
    }

    pub fn messages(&self) -> &[MessageRecord] {
        &self.messages[..self.message_len]
    }

    pub(crate) fn ensure_message_capacity(&self) -> Result<(), KernelError> {
        if self.message_len >= MESSAGES {
            Err(KernelError::MessageStoreFull)
        } else {
            Ok(())
        }
    }

    pub(crate) fn append_message(
        &mut self,
        sender: AgentId,
        recipient: AgentId,
        kind: MessageKind,
        payload: MessagePayload,
    ) -> MessageId {
        let message = MessageId::new(self.next_message);
        self.next_message += 1;
        self.messages[self.message_len] = MessageRecord {
            id: message,
            sender,
            recipient,
            kind,
            payload,
            status: MessageStatus::Pending,
        };
        self.message_len += 1;
        message
    }

    pub(crate) fn oldest_pending_message_index(&self, recipient: AgentId) -> Option<usize> {
        let mut index = 0;
        while index < self.message_len {
            let message = self.messages[index];
            if message.recipient == recipient && message.status == MessageStatus::Pending {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    pub(crate) fn find_message(&self, id: MessageId) -> Result<MessageRecord, KernelError> {
        for message in self.messages() {
            if message.id == id {
                return Ok(*message);
            }
        }

        Err(KernelError::MessageNotFound)
    }

    fn find_message_mut(&mut self, id: MessageId) -> Result<&mut MessageRecord, KernelError> {
        for message in &mut self.messages[..self.message_len] {
            if message.id == id {
                return Ok(message);
            }
        }

        Err(KernelError::MessageNotFound)
    }
}
