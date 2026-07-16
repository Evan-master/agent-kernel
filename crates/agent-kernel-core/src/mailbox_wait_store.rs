//! Blocking mailbox receive and sender-driven wakeup transitions.
//!
//! This `agent-kernel-core` module owns the fixed-capacity waiter, task,
//! execution-context, run-queue, message, and event transaction. It remains
//! deterministic and no_std; physical saved frames stay in architecture code.

use crate::{
    mailbox_wait_event::MailboxWaitEvent, AgentId, CapabilityId, EventKind, KernelCore,
    KernelError, MessageId, MessageReceiveOutcome, MessageStatus, Operation, RunQueueEntry,
    SignalKey, TaskId, TaskStatus, WaiterId, WaiterKind, WaiterRecord,
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
    pub fn receive_or_wait_message(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<MessageReceiveOutcome, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_task(task)?;
        self.ensure_authorized_for_task(
            agent,
            capability,
            task_record.resource,
            Operation::Act,
            task,
        )?;
        if task_record.status != TaskStatus::Running {
            return Err(KernelError::TaskStatusMismatch);
        }
        if task_record.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        self.ensure_agent_admitted_for_task(agent, task)?;

        if let Some(index) = self.oldest_pending_message_index(agent) {
            let message = self.messages[index];
            self.ensure_event_slots(1)?;
            self.messages[index].status = MessageStatus::Received;
            self.record_message_event(
                EventKind::MessageReceived,
                agent,
                message.sender,
                message.id,
            )?;
            return Ok(MessageReceiveOutcome::Received(message.id));
        }

        if self.waiter_len >= WAITERS {
            return Err(KernelError::WaiterStoreFull);
        }
        self.ensure_event_slots(1)?;

        let waiter = WaiterId::new(self.next_waiter);
        self.next_waiter += 1;
        self.waiters[self.waiter_len] = WaiterRecord {
            id: waiter,
            task,
            agent,
            resource: task_record.resource,
            signal: SignalKey::new(0),
            kind: WaiterKind::Mailbox,
            active: true,
        };
        self.waiter_len += 1;
        self.find_task_mut(task)?.status = TaskStatus::Waiting;
        self.set_execution_context_waiting(agent, task)?;
        self.record_mailbox_wait_event(MailboxWaitEvent::started(
            agent,
            capability,
            task_record.resource,
            task,
            waiter,
        ))?;
        Ok(MessageReceiveOutcome::Waiting(waiter))
    }

    pub(crate) fn find_mailbox_waiter_index(&self, recipient: AgentId) -> Option<usize> {
        let mut index = 0;
        while index < self.waiter_len {
            let waiter = self.waiters[index];
            if waiter.active && waiter.kind == WaiterKind::Mailbox && waiter.agent == recipient {
                if let Ok(task) = self.find_task(waiter.task) {
                    if task.status == TaskStatus::Waiting && task.assignee == Some(recipient) {
                        return Some(index);
                    }
                }
            }
            index += 1;
        }
        None
    }

    pub(crate) fn wake_mailbox_waiter(
        &mut self,
        waiter_index: usize,
        sender: AgentId,
        message: MessageId,
    ) -> Result<(), KernelError> {
        let waiter = self.waiters[waiter_index];
        self.waiters[waiter_index].active = false;
        self.find_task_mut(waiter.task)?.status = TaskStatus::Accepted;
        self.set_execution_context_idle(waiter.agent)?;
        self.run_queue[self.run_queue_len] = RunQueueEntry {
            task: waiter.task,
            agent: waiter.agent,
        };
        self.run_queue_len += 1;
        self.record_mailbox_wait_event(MailboxWaitEvent::woken(
            sender,
            waiter.agent,
            waiter.resource,
            waiter.task,
            waiter.id,
            message,
        ))?;
        Ok(())
    }
}
