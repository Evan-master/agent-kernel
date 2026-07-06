//! Deterministic scheduler tick accounting.
//!
//! This module belongs to `agent-kernel-core`. It advances running tasks by
//! explicit supervisor/HAL ticks, records auditable progress, and preempts tasks
//! back into the fixed-capacity run queue when their assigned quantum expires.

use crate::{
    AgentId, Event, EventKind, KernelCore, KernelError, RunQueueEntry, TaskId, TaskStatus,
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
    >
{
    pub fn tick_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_task(task)?;
        if task_record.status != TaskStatus::Running || task_record.assignee != Some(agent) {
            return Err(KernelError::TaskNotRunnable);
        }
        if task_record.quantum_remaining == 0 {
            return Err(KernelError::TaskQuantumInvalid);
        }
        if task_record.quantum_remaining == 1 {
            self.ensure_not_queued(task)?;
            self.ensure_run_queue_capacity()?;
        }
        self.ensure_scheduler_event_capacity()?;

        let new_ticks = task_record.run_ticks + 1;
        let remaining = task_record.quantum_remaining - 1;
        let kind = if remaining == 0 {
            EventKind::TaskQuantumExpired
        } else {
            EventKind::TaskTicked
        };

        let task_ref = self.find_task_mut(task)?;
        task_ref.run_ticks = new_ticks;
        task_ref.quantum_remaining = remaining;
        if remaining == 0 {
            task_ref.status = TaskStatus::Accepted;
            self.run_queue[self.run_queue_len] = RunQueueEntry { task, agent };
            self.run_queue_len += 1;
        }

        self.record_scheduler_event(
            kind,
            agent,
            task,
            task_record.resource,
            Some(new_ticks),
            Some(remaining),
        )
    }
}
