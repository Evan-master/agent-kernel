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

    pub fn dispatch_next(&mut self, agent: AgentId) -> Result<TaskId, KernelError> {
        self.dispatch_next_with_quantum(agent, 1)
    }

    pub fn dispatch_next_with_quantum(
        &mut self,
        agent: AgentId,
        quantum: u64,
    ) -> Result<TaskId, KernelError> {
        if quantum == 0 {
            return Err(KernelError::TaskQuantumInvalid);
        }
        self.ensure_agent_active(agent)?;
        if self.run_queue_len == 0 {
            return Err(KernelError::RunQueueEmpty);
        }

        let entry = self.run_queue[0];
        if entry.agent != agent {
            return Err(KernelError::TaskNotRunnable);
        }
        self.dispatch_ready_entry(entry, quantum)?;
        Ok(entry.task)
    }

    pub fn dispatch_next_ready_with_quantum(
        &mut self,
        quantum: u64,
    ) -> Result<RunQueueEntry, KernelError> {
        if quantum == 0 {
            return Err(KernelError::TaskQuantumInvalid);
        }
        if self.run_queue_len == 0 {
            return Err(KernelError::RunQueueEmpty);
        }

        let entry = self.run_queue[0];
        self.dispatch_ready_entry(entry, quantum)?;
        Ok(entry)
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

    fn find_runnable_task(&self, agent: AgentId, task: TaskId) -> Result<Task, KernelError> {
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

    fn dispatch_ready_entry(
        &mut self,
        entry: RunQueueEntry,
        quantum: u64,
    ) -> Result<(), KernelError> {
        self.ensure_agent_active(entry.agent)?;
        self.ensure_execution_context_idle(entry.agent)?;
        let task_record = self.find_runnable_task(entry.agent, entry.task)?;
        self.ensure_agent_admitted_for_task(entry.agent, entry.task)?;
        self.ensure_scheduler_event_capacity()?;

        self.shift_run_queue_left();
        let task = self.find_task_mut(entry.task)?;
        task.status = TaskStatus::Running;
        task.quantum_remaining = quantum;
        self.set_execution_context_running(
            entry.agent,
            entry.task,
            task_record.run_ticks,
            quantum,
        )?;
        self.record_scheduler_event(
            EventKind::TaskDispatched,
            entry.agent,
            entry.task,
            task_record.resource,
            None,
            Some(quantum),
        )?;
        Ok(())
    }

    fn shift_run_queue_left(&mut self) {
        let last = self.run_queue_len - 1;
        let mut index = 0;
        while index < last {
            self.run_queue[index] = self.run_queue[index + 1];
            index += 1;
        }
        self.run_queue[last] = RunQueueEntry::empty();
        self.run_queue_len -= 1;
    }
}
