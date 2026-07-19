//! Driver Invocation runtime syscall facade.
//!
//! This no_std facade exposes dispatch, tick, completion, and read-only
//! invocation inspection while all queue, admission, and execution-context
//! mutation remains inside `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, DriverInvocationId, DriverInvocationRecord, Event, KernelError,
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
    pub fn sys_dispatch_next_driver_invocation(
        &mut self,
        driver: AgentId,
        quantum: u64,
    ) -> Result<DriverInvocationId, KernelError> {
        self.core.dispatch_next_driver_invocation(driver, quantum)
    }

    pub fn sys_tick_driver_invocation(
        &mut self,
        driver: AgentId,
        invocation: DriverInvocationId,
    ) -> Result<Event, KernelError> {
        self.core.tick_driver_invocation(driver, invocation)
    }

    pub fn sys_complete_driver_invocation(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        invocation: DriverInvocationId,
    ) -> Result<Event, KernelError> {
        self.core
            .complete_driver_invocation(driver, capability, invocation)
    }

    pub fn driver_invocations(&self) -> &[DriverInvocationRecord] {
        self.core.driver_invocations()
    }
}
