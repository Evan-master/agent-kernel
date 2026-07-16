//! Mailbox wait and wake audit event builders.
//!
//! This `agent-kernel-core` module converts typed mailbox wait fields into the
//! fixed replayable Event shape. It performs no state transition or capacity
//! reservation; callers must reserve event slots before mutation.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, MessageId, ResourceId,
    TaskId, WaiterId,
};

pub(crate) struct MailboxWaitEvent {
    kind: EventKind,
    agent: AgentId,
    capability: Option<CapabilityId>,
    resource: ResourceId,
    task: TaskId,
    waiter: WaiterId,
    target_agent: Option<AgentId>,
    message: Option<MessageId>,
}

impl MailboxWaitEvent {
    pub(crate) const fn started(
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        task: TaskId,
        waiter: WaiterId,
    ) -> Self {
        Self {
            kind: EventKind::MessageWaitStarted,
            agent,
            capability: Some(capability),
            resource,
            task,
            waiter,
            target_agent: None,
            message: None,
        }
    }

    pub(crate) const fn woken(
        sender: AgentId,
        recipient: AgentId,
        resource: ResourceId,
        task: TaskId,
        waiter: WaiterId,
        message: MessageId,
    ) -> Self {
        Self {
            kind: EventKind::MessageWaitWoken,
            agent: sender,
            capability: None,
            resource,
            task,
            waiter,
            target_agent: Some(recipient),
            message: Some(message),
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
    pub(crate) fn record_mailbox_wait_event(
        &mut self,
        fields: MailboxWaitEvent,
    ) -> Result<Event, KernelError> {
        let intent = self.find_task(fields.task)?.intent;
        let mut event = Event::empty();
        event.agent = fields.agent;
        event.kind = fields.kind;
        event.resource = Some(fields.resource);
        event.capability = fields.capability;
        event.intent = Some(intent);
        event.task = Some(fields.task);
        event.waiter = Some(fields.waiter);
        event.target_agent = fields.target_agent;
        event.message = fields.message;
        self.record(event)
    }
}
