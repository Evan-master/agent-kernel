//! Native mailbox message event construction.
//!
//! This core-layer module builds deterministic fixed-field send, receive, and
//! acknowledgement events. Message state transitions and capacity checks stay
//! in the mailbox stores.

use crate::{AgentId, Event, EventKind, KernelCore, KernelError, MessageId};

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
    pub(crate) fn record_message_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        target_agent: AgentId,
        message: MessageId,
    ) -> Result<Event, KernelError> {
        let mut event = Event::empty();
        event.agent = agent;
        event.kind = kind;
        event.message = Some(message);
        event.target_agent = Some(target_agent);
        self.record(event)
    }
}
