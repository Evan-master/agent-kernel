//! Authenticated retirement of a terminal Task prefix.
//!
//! This no_std core module validates Supervisor authority, terminal Intent
//! state, live references, and Event capacity before reclaiming fixed Task
//! slots. It preserves Task ID monotonicity and invalidates dispatch permits.

use crate::task_event::task_compaction_event;
use crate::{
    AgentEntryKind, AgentId, CapabilityId, IntentStatus, KernelCore, KernelError, MessageStatus,
    NamespaceObject, Operation, Task, TaskCompaction, TaskId, TaskStatus,
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
        const RUNTIME_ADMISSIONS: usize,
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
        RUNTIME_ADMISSIONS,
    >
{
    pub fn compact_task_prefix(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        through: TaskId,
    ) -> Result<TaskCompaction, KernelError> {
        let actor_entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if actor_entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let through_index = self
            .tasks()
            .iter()
            .position(|task| task.id == through)
            .ok_or(KernelError::TaskNotFound)?;
        let count = through_index + 1;
        for task in self.tasks()[..count].iter().copied() {
            self.ensure_task_compaction_ready(task)?;
            self.ensure_authorized(actor, authority, task.resource, Operation::Rollback)?;
        }
        self.ensure_event_slots(count)?;

        let previous = self.tasks;
        let remaining = self.task_len - count;
        self.tasks[..remaining].copy_from_slice(&previous[count..self.task_len]);
        for index in remaining..self.task_len {
            self.tasks[index] = Task::empty();
        }
        self.task_len = remaining;
        self.task_generation += 1;
        for task in previous[..count].iter().copied() {
            self.record(task_compaction_event(task, actor, authority))?;
        }

        Ok(TaskCompaction::new(previous[0].id, through, count))
    }

    fn ensure_task_compaction_ready(&self, task: Task) -> Result<(), KernelError> {
        let intent = self.find_intent(task.intent)?;
        let terminal_state_matches = matches!(
            (task.status, intent.status),
            (TaskStatus::Verified, IntentStatus::Fulfilled)
                | (TaskStatus::Cancelled, IntentStatus::Cancelled)
        );
        if !terminal_state_matches {
            return Err(KernelError::TaskCompactionNotReady);
        }

        let referenced = self.run_queue().iter().any(|entry| entry.task == task.id)
            || self
                .execution_contexts()
                .iter()
                .any(|context| context.task == Some(task.id))
            || self
                .waiters()
                .iter()
                .any(|waiter| waiter.active && waiter.task == task.id)
            || self
                .runtime_admissions()
                .iter()
                .any(|admission| admission.task == task.id)
            || self
                .namespace_entries()
                .iter()
                .any(|entry| entry.object == NamespaceObject::Task(task.id))
            || self.messages().iter().any(|message| {
                message.payload.task == Some(task.id)
                    && message.status != MessageStatus::Acknowledged
            });
        if referenced {
            Err(KernelError::TaskCompactionReferenced)
        } else {
            Ok(())
        }
    }
}
