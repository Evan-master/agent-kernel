//! Fixed-capacity task lookup helpers.
//!
//! This module belongs to `agent-kernel-core`. It owns read and mutable lookup
//! behavior for task records plus shared lifecycle status validation, keeping
//! task transition code focused on state changes and event consequences.

use crate::{KernelCore, KernelError, Task, TaskId, TaskStatus};

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
    >
{
    pub fn tasks(&self) -> &[Task] {
        &self.tasks[..self.task_len]
    }

    pub(crate) fn find_task(&self, id: TaskId) -> Result<Task, KernelError> {
        self.tasks()
            .iter()
            .find(|task| task.id == id)
            .copied()
            .ok_or(KernelError::TaskNotFound)
    }

    pub(crate) fn find_task_mut(&mut self, id: TaskId) -> Result<&mut Task, KernelError> {
        self.tasks[..self.task_len]
            .iter_mut()
            .find(|task| task.id == id)
            .ok_or(KernelError::TaskNotFound)
    }
}

pub(crate) fn ensure_status(
    current: TaskStatus,
    allowed: &[TaskStatus],
) -> Result<(), KernelError> {
    allowed
        .contains(&current)
        .then_some(())
        .ok_or(KernelError::TaskStatusMismatch)
}
