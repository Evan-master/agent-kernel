//! Wait signal syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes task wait and signal
//! emission operations as boundary methods while keeping waiter storage and run
//! queue wakeup mutation inside `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, KernelError, ResourceId, SignalKey, SignalOutcome, TaskId, WaiterId,
    WaiterRecord,
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
    >
{
    pub fn sys_wait_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        resource: ResourceId,
        signal: SignalKey,
    ) -> Result<WaiterId, KernelError> {
        self.core
            .wait_task(agent, capability, task, resource, signal)
    }

    pub fn sys_emit_signal(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        signal: SignalKey,
    ) -> Result<SignalOutcome, KernelError> {
        self.core.emit_signal(agent, capability, resource, signal)
    }

    pub fn waiters(&self) -> &[WaiterRecord] {
        self.core.waiters()
    }
}
