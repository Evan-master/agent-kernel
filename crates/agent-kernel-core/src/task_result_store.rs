//! Task-scoped fixed-width result submission.
//!
//! This module belongs to `agent-kernel-core`. It authorizes one replayable
//! result for the currently running assignee without changing scheduler state.

use crate::{
    task_lookup::ensure_status, AgentId, CapabilityId, Event, KernelCore, KernelError, Operation,
    TaskId, TaskResult, TaskStatus,
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
    pub fn submit_task_result(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        result: TaskResult,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let current = self.find_task(task)?;
        self.ensure_authorized_for_task(agent, capability, current.resource, Operation::Act, task)?;
        ensure_status(current.status, &[TaskStatus::Running])?;
        if current.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        self.ensure_agent_admitted_for_task(agent, task)?;
        if current.result.is_some() {
            return Err(KernelError::TaskResultAlreadySubmitted);
        }
        self.ensure_event_slots(1)?;

        self.find_task_mut(task)?.result = Some(result);
        self.record_task_result_event(
            crate::EventKind::TaskResultSubmitted,
            agent,
            capability,
            task,
            result,
        )
    }

    pub fn inspect_task_result(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Verify)?;
        ensure_status(current.status, &[TaskStatus::Completed])?;
        self.ensure_agent_admitted_for_verification(agent, task)?;
        let result = current.result.ok_or(KernelError::TaskResultMissing)?;
        self.ensure_event_slots(1)?;

        self.record_task_result_event(
            crate::EventKind::TaskResultInspected,
            agent,
            capability,
            task,
            result,
        )
    }
}
