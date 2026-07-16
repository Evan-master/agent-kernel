//! Fixed-capacity FIFO task scheduler; tick accounting lives in `scheduler_tick`.

use crate::{
    AgentId, Event, EventKind, KernelCore, KernelError, RunQueueEntry, Task, TaskId, TaskStatus,
};

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
    KernelCore<
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
    pub fn enqueue_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_runnable_task(agent, task)?;
        self.ensure_agent_admitted_for_task(agent, task)?;
        self.ensure_not_queued(task)?;
        self.ensure_run_queue_capacity()?;
        self.ensure_scheduler_event_capacity()?;

        self.run_queue[self.run_queue_len] = RunQueueEntry { task, agent };
        self.run_queue_len += 1;
        self.record_scheduler_event(
            EventKind::TaskQueued,
            agent,
            task,
            task_record.resource,
            None,
            None,
        )
    }

    pub fn yield_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_task(task)?;
        if task_record.status != TaskStatus::Running || task_record.assignee != Some(agent) {
            return Err(KernelError::TaskNotRunnable);
        }
        self.ensure_agent_admitted_for_task(agent, task)?;
        self.ensure_not_queued(task)?;
        self.ensure_run_queue_capacity()?;
        self.ensure_scheduler_event_capacity()?;

        self.find_task_mut(task)?.status = TaskStatus::Accepted;
        self.set_execution_context_idle(agent)?;
        self.run_queue[self.run_queue_len] = RunQueueEntry { task, agent };
        self.run_queue_len += 1;
        self.record_scheduler_event(
            EventKind::TaskYielded,
            agent,
            task,
            task_record.resource,
            None,
            None,
        )
    }

    pub fn run_queue(&self) -> &[RunQueueEntry] {
        &self.run_queue[..self.run_queue_len]
    }

    pub(crate) fn find_runnable_task(
        &self,
        agent: AgentId,
        task: TaskId,
    ) -> Result<Task, KernelError> {
        let task_record = self.find_task(task)?;
        if task_record.status != TaskStatus::Accepted || task_record.assignee != Some(agent) {
            return Err(KernelError::TaskNotRunnable);
        }
        Ok(task_record)
    }

    pub(crate) fn ensure_not_queued(&self, task: TaskId) -> Result<(), KernelError> {
        (!self.run_queue().iter().any(|entry| entry.task == task))
            .then_some(())
            .ok_or(KernelError::TaskAlreadyQueued)
    }

    pub(crate) fn ensure_run_queue_capacity(&self) -> Result<(), KernelError> {
        (self.run_queue_len < RUN_QUEUE)
            .then_some(())
            .ok_or(KernelError::RunQueueFull)
    }

    pub(crate) fn ensure_scheduler_event_capacity(&self) -> Result<(), KernelError> {
        (self.event_len < EVENTS)
            .then_some(())
            .ok_or(KernelError::EventLogFull)
    }
}
