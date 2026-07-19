//! Memory cell syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes native remember/recall
//! state operations as syscall-style methods while keeping fixed-capacity cell
//! mutation inside `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, KernelError, MemoryCellId, MemoryCellRecord, MemoryValue,
    ResourceId,
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
    pub fn sys_create_memory_cell(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        value: MemoryValue,
    ) -> Result<MemoryCellId, KernelError> {
        self.core
            .create_memory_cell(agent, capability, resource, value)
    }

    pub fn sys_recall_memory_cell(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        cell: MemoryCellId,
    ) -> Result<MemoryValue, KernelError> {
        self.core.recall_memory_cell(agent, capability, cell)
    }

    pub fn sys_remember_memory_cell(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        cell: MemoryCellId,
        value: MemoryValue,
    ) -> Result<Event, KernelError> {
        self.core
            .remember_memory_cell(agent, capability, cell, value)
    }

    pub fn memory_cells(&self) -> &[MemoryCellRecord] {
        self.core.memory_cells()
    }
}
