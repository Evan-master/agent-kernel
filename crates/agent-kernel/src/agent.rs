//! Agent lifecycle and execution context facade methods.
//!
//! This module belongs to `agent-kernel`. It exposes agent registration,
//! lifecycle, and read-only execution context inspection without letting
//! callers mutate `agent-kernel-core` stores directly.

use agent_kernel_core::{
    AgentEntryKind, AgentEntryRecord, AgentExecutionContext, AgentId, AgentImageDigest,
    AgentImageId, AgentImageKind, AgentImageRecord, AgentRecord, CapabilityId, Event, IntentId,
    KernelError, ResourceId, TaskId,
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
        const AGENT_IMAGES: usize,
        const DRIVER_BINDINGS: usize,
        const DEVICE_EVENTS: usize,
        const DRIVER_COMMANDS: usize,
        const DRIVER_INVOCATIONS: usize,
        const RUNTIME_ADMISSIONS: usize,
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
        AGENT_IMAGES,
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
        RUNTIME_ADMISSIONS,
    >
{
    pub fn sys_register_agent(&mut self, agent: AgentId) -> Result<Event, KernelError> {
        self.core.register_agent(agent)
    }

    pub fn sys_register_agent_image(
        &mut self,
        owner: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: AgentImageKind,
        digest: AgentImageDigest,
        abi_version: u16,
        entry_version: u16,
    ) -> Result<AgentImageId, KernelError> {
        self.core.register_agent_image(
            owner,
            capability,
            resource,
            kind,
            digest,
            abi_version,
            entry_version,
        )
    }

    pub fn sys_verify_agent_image(
        &mut self,
        owner: AgentId,
        capability: CapabilityId,
        image: AgentImageId,
    ) -> Result<Event, KernelError> {
        self.core.verify_agent_image(owner, capability, image)
    }

    pub fn sys_retire_agent_image(
        &mut self,
        owner: AgentId,
        capability: CapabilityId,
        image: AgentImageId,
    ) -> Result<Event, KernelError> {
        self.core.retire_agent_image(owner, capability, image)
    }

    pub fn sys_launch_agent(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        image: AgentImageId,
        kind: AgentEntryKind,
        intent: Option<IntentId>,
    ) -> Result<Event, KernelError> {
        self.core
            .launch_agent(agent, capability, resource, image, kind, intent)
    }

    pub fn sys_launch_task_agent(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        image: AgentImageId,
        kind: AgentEntryKind,
    ) -> Result<Event, KernelError> {
        self.core
            .launch_task_agent(agent, capability, task, image, kind)
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

    pub const fn agent_capacity(&self) -> usize {
        self.core.agent_capacity()
    }

    pub const fn agent_count(&self) -> usize {
        self.core.agent_count()
    }

    pub fn agent_entries(&self) -> &[AgentEntryRecord] {
        self.core.agent_entries()
    }

    pub const fn agent_entry_capacity(&self) -> usize {
        self.core.agent_entry_capacity()
    }

    pub const fn agent_entry_count(&self) -> usize {
        self.core.agent_entry_count()
    }

    pub fn agent_images(&self) -> &[AgentImageRecord] {
        self.core.agent_images()
    }

    pub fn agent_entry(&self, agent: AgentId) -> Result<AgentEntryRecord, KernelError> {
        self.core.agent_entry(agent)
    }

    pub fn agent_image(&self, image: AgentImageId) -> Result<AgentImageRecord, KernelError> {
        self.core.agent_image(image)
    }

    pub fn execution_contexts(&self) -> &[AgentExecutionContext] {
        self.core.execution_contexts()
    }

    pub fn execution_context(&self, agent: AgentId) -> Result<AgentExecutionContext, KernelError> {
        self.core.execution_context(agent)
    }
}
