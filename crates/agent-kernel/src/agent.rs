//! Agent lifecycle and execution context facade methods.
//!
//! This module belongs to `agent-kernel`. It exposes agent registration,
//! lifecycle, and read-only execution context inspection without letting
//! callers mutate `agent-kernel-core` stores directly.

use agent_kernel_core::{
    AgentEntryKind, AgentEntryRecord, AgentExecutionContext, AgentId, AgentRecord, CapabilityId,
    Event, IntentId, KernelError, ResourceId,
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
        const WAITERS: usize,
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
        WAITERS,
    >
{
    pub fn sys_register_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        self.core.register_agent(agent)
    }

    pub fn sys_launch_agent(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: AgentEntryKind,
        intent: Option<IntentId>,
    ) -> Result<Event, KernelError> {
        self.core
            .launch_agent(agent, capability, resource, kind, intent)
    }

    pub fn sys_suspend_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        self.core.suspend_agent(agent)
    }

    pub fn sys_resume_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        self.core.resume_agent(agent)
    }

    pub fn sys_retire_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        self.core.retire_agent(agent)
    }

    pub fn agents(&self) -> &[AgentRecord] {
        self.core.agents()
    }

    pub fn agent_entries(&self) -> &[AgentEntryRecord] {
        self.core.agent_entries()
    }

    pub fn agent_entry(&self, agent: AgentId) -> Result<AgentEntryRecord, KernelError> {
        self.core.agent_entry(agent)
    }

    pub fn execution_contexts(&self) -> &[AgentExecutionContext] {
        self.core.execution_contexts()
    }
}
