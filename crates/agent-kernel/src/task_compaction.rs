//! Task Store inspection and authenticated compaction facade.
//!
//! This no_std boundary forwards typed Task retirement requests to the
//! deterministic core without exposing mutable store internals.

use agent_kernel_core::{AgentId, CapabilityId, KernelError, Task, TaskCompaction, TaskId};

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
    pub fn sys_compact_task_prefix(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        through: TaskId,
    ) -> Result<TaskCompaction, KernelError> {
        self.core.compact_task_prefix(actor, authority, through)
    }

    pub const fn task_capacity(&self) -> usize {
        self.core.task_capacity()
    }

    pub fn task(&self, task: TaskId) -> Result<Task, KernelError> {
        self.core.task(task)
    }
}
