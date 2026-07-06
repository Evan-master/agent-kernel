//! Scheduler syscall facade.
//!
//! This module belongs to `agent-kernel`. It exposes scheduler operations as
//! boundary methods while keeping run queue mutation inside `agent-kernel-core`.

use agent_kernel_core::{AgentId, Event, KernelError, RunQueueEntry, TaskId};

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
    >
{
    pub fn sys_enqueue_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        self.core.enqueue_task(agent, task)
    }

    pub fn sys_dispatch_next(&mut self, agent: AgentId) -> Result<TaskId, KernelError> {
        self.core.dispatch_next(agent)
    }

    pub fn sys_yield_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        self.core.yield_task(agent, task)
    }

    pub fn run_queue(&self) -> &[RunQueueEntry] {
        self.core.run_queue()
    }
}
