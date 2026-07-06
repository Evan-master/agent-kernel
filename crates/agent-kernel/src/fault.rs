//! Task fault syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes task fault and recovery
//! operations as boundary methods while keeping fixed-capacity fault mutation
//! inside `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, FaultId, FaultKind, FaultRecord, KernelError, TaskId,
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
    >
{
    pub fn sys_fault_task(
        &mut self,
        agent: AgentId,
        task: TaskId,
        kind: FaultKind,
        detail: u64,
    ) -> Result<FaultId, KernelError> {
        self.core.fault_task(agent, task, kind, detail)
    }

    pub fn sys_recover_faulted_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.core.recover_faulted_task(agent, capability, task)
    }

    pub fn faults(&self) -> &[FaultRecord] {
        self.core.faults()
    }
}
