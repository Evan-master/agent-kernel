//! Two-phase task dispatch validation and commit.
//!
//! This core module owns read-only dispatch permits, stale-head rejection, and
//! the one atomic FIFO-to-Running mutation. Architecture readiness checks occur
//! between prepare and commit but never enter this deterministic no_std layer.

use crate::{
    AgentId, EventKind, KernelCore, KernelError, RunQueueEntry, Task, TaskDispatchPermit, TaskId,
    TaskStatus,
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
        let permit = self.prepare_next_ready_dispatch_with_quantum(quantum)?;
        self.commit_ready_dispatch(permit)
    }

    pub fn prepare_next_ready_dispatch_with_quantum(
        &self,
        quantum: u64,
    ) -> Result<TaskDispatchPermit, KernelError> {
        if quantum == 0 {
            return Err(KernelError::TaskQuantumInvalid);
        }
        if self.run_queue_len == 0 {
            return Err(KernelError::RunQueueEmpty);
        }

        let entry = self.run_queue[0];
        self.ensure_dispatch_ready_entry(entry)?;
        Ok(TaskDispatchPermit::new(
            entry,
            quantum,
            self.task_generation,
        ))
    }

    pub fn commit_ready_dispatch(
        &mut self,
        permit: TaskDispatchPermit,
    ) -> Result<RunQueueEntry, KernelError> {
        if permit.generation() != self.task_generation {
            return Err(KernelError::TaskDispatchPermitStale);
        }
        let entry = permit.entry();
        if self.run_queue_len == 0 || self.run_queue[0] != entry {
            return Err(KernelError::TaskNotRunnable);
        }
        self.dispatch_ready_entry(entry, permit.quantum())?;
        Ok(entry)
    }

    fn ensure_dispatch_ready_entry(&self, entry: RunQueueEntry) -> Result<Task, KernelError> {
        self.ensure_agent_active(entry.agent)?;
        self.ensure_execution_context_idle(entry.agent)?;
        let task = self.find_runnable_task(entry.agent, entry.task)?;
        self.ensure_agent_admitted_for_task(entry.agent, entry.task)?;
        self.ensure_scheduler_event_capacity()?;
        Ok(task)
    }

    fn dispatch_ready_entry(
        &mut self,
        entry: RunQueueEntry,
        quantum: u64,
    ) -> Result<(), KernelError> {
        let task_record = self.ensure_dispatch_ready_entry(entry)?;

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
