//! Mailbox syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes native message IPC as
//! syscall-style methods while keeping fixed-capacity mailbox mutation inside
//! `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, Event, KernelError, MessageId, MessageKind, MessagePayload, MessageRecord,
};

use crate::AgentKernel;

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
    AgentKernel<
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
    pub fn sys_send_message(
        &mut self,
        sender: AgentId,
        recipient: AgentId,
        kind: MessageKind,
        payload: MessagePayload,
    ) -> Result<MessageId, KernelError> {
        self.core.send_message(sender, recipient, kind, payload)
    }

    pub fn sys_receive_message(&mut self, agent: AgentId) -> Result<MessageId, KernelError> {
        self.core.receive_message(agent)
    }

    pub fn sys_acknowledge_message(
        &mut self,
        agent: AgentId,
        message: MessageId,
    ) -> Result<Event, KernelError> {
        self.core.acknowledge_message(agent, message)
    }

    pub fn messages(&self) -> &[MessageRecord] {
        self.core.messages()
    }
}
