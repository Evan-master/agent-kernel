//! Fixed-capacity FIFO task scheduler.
//!
//! This module belongs to `agent-kernel-core`. It owns FIFO enqueue, dispatch,
//! and yield behavior for accepted or running tasks. Deterministic tick
//! accounting lives in `scheduler_tick`.

use crate::{
    AgentId, Event, EventKind, KernelCore, KernelError, OperationSet, ResourceId, RunQueueEntry,
    Task, TaskId, TaskStatus, VerificationRequirement,
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
    >
{
    pub fn enqueue_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_runnable_task(agent, task)?;
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
        let task_record = self.find_runnable_task(agent, entry.task)?;
        self.ensure_scheduler_event_capacity()?;

        self.shift_run_queue_left();
        let task = self.find_task_mut(entry.task)?;
        task.status = TaskStatus::Running;
        task.quantum_remaining = quantum;
        self.record_scheduler_event(
            EventKind::TaskDispatched,
            agent,
            entry.task,
            task_record.resource,
            None,
            Some(quantum),
        )?;
        Ok(entry.task)
    }

    pub fn yield_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_task(task)?;
        if task_record.status != TaskStatus::Running || task_record.assignee != Some(agent) {
            return Err(KernelError::TaskNotRunnable);
        }
        self.ensure_not_queued(task)?;
        self.ensure_run_queue_capacity()?;
        self.ensure_scheduler_event_capacity()?;

        self.find_task_mut(task)?.status = TaskStatus::Accepted;
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
        if self.run_queue().iter().any(|entry| entry.task == task) {
            Err(KernelError::TaskAlreadyQueued)
        } else {
            Ok(())
        }
    }

    pub(crate) fn ensure_run_queue_capacity(&self) -> Result<(), KernelError> {
        if self.run_queue_len >= RUN_QUEUE {
            Err(KernelError::RunQueueFull)
        } else {
            Ok(())
        }
    }

    pub(crate) fn ensure_scheduler_event_capacity(&self) -> Result<(), KernelError> {
        if self.event_len >= EVENTS {
            Err(KernelError::EventLogFull)
        } else {
            Ok(())
        }
    }

    pub(crate) fn record_scheduler_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        task: TaskId,
        resource: ResourceId,
        task_ticks: Option<u64>,
        task_quantum: Option<u64>,
    ) -> Result<Event, KernelError> {
        let task_record = self.find_task(task)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind,
            resource: Some(resource),
            capability: None,
            source_capability: None,
            intent: Some(task_record.intent),
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: Some(task),
            task_ticks,
            task_quantum,
            target_agent: None,
        })
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
