//! Task completion, verification, and cancellation transitions.
//!
//! This module belongs to `agent-kernel-core`. It owns terminal task lifecycle
//! transitions, intent finalization, and execution-context cleanup.

use crate::{
    intent_event::IntentTaskEventKind, task_lookup::ensure_status, AgentId, CapabilityId, Event,
    EventKind, IntentStatus, KernelCore, KernelError, Operation, TaskId, TaskStatus,
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
    >
{
    pub fn complete_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let current = self.find_task(task)?;
        self.ensure_authorized_for_task(agent, capability, current.resource, Operation::Act, task)?;
        ensure_status(current.status, &[TaskStatus::Running])?;
        if current.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        self.ensure_agent_admitted_for_task(agent, task)?;
        self.ensure_event_slots(1)?;

        self.find_task_mut(task)?.status = TaskStatus::Completed;
        self.set_execution_context_idle(agent)?;
        self.record_task_event(
            EventKind::TaskCompleted,
            agent,
            Some(capability),
            task,
            None,
        )
    }

    pub fn verify_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Verify)?;
        ensure_status(current.status, &[TaskStatus::Completed])?;
        self.ensure_intent_status(current.intent, IntentStatus::Bound)?;
        self.ensure_event_slots(2)?;

        self.find_task_mut(task)?.status = TaskStatus::Verified;
        let event =
            self.record_task_event(EventKind::TaskVerified, agent, Some(capability), task, None)?;
        self.set_intent_status(current.intent, IntentStatus::Fulfilled)?;
        self.record_intent_task_event(IntentTaskEventKind::Fulfilled, agent, task)?;
        Ok(event)
    }

    pub fn cancel_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Rollback)?;
        ensure_status(
            current.status,
            &[
                TaskStatus::Created,
                TaskStatus::Delegated,
                TaskStatus::Accepted,
                TaskStatus::Running,
                TaskStatus::Completed,
            ],
        )?;
        self.ensure_intent_status(current.intent, IntentStatus::Bound)?;
        self.ensure_event_slots(2)?;

        self.find_task_mut(task)?.status = TaskStatus::Cancelled;
        self.clear_execution_context_for_task(task);
        let event = self.record_task_event(
            EventKind::TaskCancelled,
            agent,
            Some(capability),
            task,
            None,
        )?;
        self.set_intent_status(current.intent, IntentStatus::Cancelled)?;
        self.record_intent_task_event(IntentTaskEventKind::Cancelled, agent, task)?;
        Ok(event)
    }
}
