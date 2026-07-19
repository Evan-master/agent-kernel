//! Public facade for managed Agent Record and execution-context retirement.
//!
//! This no_std boundary forwards authenticated lifecycle requests to Core and
//! exposes deterministic retirement-floor inspection without Store mutation.

use agent_kernel_core::{AgentId, AgentRecordRetirement, CapabilityId, KernelError};

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
    pub fn sys_retire_agent_record(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: AgentId,
    ) -> Result<AgentRecordRetirement, KernelError> {
        self.core.retire_agent_record(actor, authority, target)
    }

    pub const fn retired_agent_floor(&self) -> AgentId {
        self.core.retired_agent_floor()
    }
}
