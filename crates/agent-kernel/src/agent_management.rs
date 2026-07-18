//! Capability-authorized Agent management facade.
//!
//! This facade-layer module exposes managed identity registration and bounded
//! lifecycle transitions. All authorization, quiescence, mutation, and event
//! decisions remain in `agent-kernel-core`.

use agent_kernel_core::{AgentId, CapabilityId, Event, KernelError, ResourceId};

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
    >
{
    pub fn sys_register_managed_agent(
        &mut self,
        manager: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        target: AgentId,
    ) -> Result<Event, KernelError> {
        self.core
            .register_managed_agent(manager, capability, resource, target)
    }

    pub fn sys_suspend_managed_agent(
        &mut self,
        actor: AgentId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Result<Event, KernelError> {
        self.core.suspend_managed_agent(actor, capability, target)
    }

    pub fn sys_resume_managed_agent(
        &mut self,
        actor: AgentId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Result<Event, KernelError> {
        self.core.resume_managed_agent(actor, capability, target)
    }

    pub fn sys_retire_managed_agent(
        &mut self,
        actor: AgentId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Result<Event, KernelError> {
        self.core.retire_managed_agent(actor, capability, target)
    }
}
