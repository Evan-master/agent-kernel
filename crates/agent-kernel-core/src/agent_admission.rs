//! Runtime admission checks for launched agents.
//!
//! This core-layer module turns launch entries into the execution boundary for
//! task runtime mutation. It depends on task lookup, launch-entry lookup, and
//! capability authorization, and must stay no_std, deterministic, and free of
//! host I/O. Keep failed admission checks side-effect-free so rejected runtime
//! operations remain invisible in the event log.

use crate::{AgentId, KernelCore, KernelError, Operation, TaskId};

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
    >
{
    pub(crate) fn ensure_agent_admitted_for_task(
        &self,
        agent: AgentId,
        task: TaskId,
    ) -> Result<(), KernelError> {
        let task_record = self.find_task(task)?;
        let entry = self
            .find_agent_entry(agent)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if entry.resource != task_record.resource {
            return Err(KernelError::ResourceMismatch);
        }
        if let Some(entry_task) = entry.task {
            if entry_task != task {
                return Err(KernelError::AgentEntryScopeMismatch);
            }
            self.ensure_authorized_for_task(
                agent,
                entry.capability,
                task_record.resource,
                Operation::Act,
                task,
            )
        } else {
            self.ensure_authorized(
                agent,
                entry.capability,
                task_record.resource,
                Operation::Act,
            )
        }
    }
}
