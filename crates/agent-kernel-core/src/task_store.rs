//! Fixed-capacity task store and lifecycle transitions.
//!
//! This module belongs to `agent-kernel-core`. It owns task allocation,
//! lifecycle validation, capability-gated task mutation, and task event
//! recording. It performs no allocation or host I/O.

use crate::{
    intent_event::IntentTaskEventKind, AgentId, CapabilityId, Event, EventKind, IntentId,
    IntentStatus, KernelCore, KernelError, Operation, OperationSet, Task, TaskId, TaskStatus,
};

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    >
    KernelCore<
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
    >
{
    pub fn create_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        intent: IntentId,
    ) -> Result<TaskId, KernelError> {
        let intent_record = self.find_intent(intent)?;
        if intent_record.owner != agent {
            return Err(KernelError::IntentAgentMismatch);
        }
        if intent_record.status != IntentStatus::Declared {
            return Err(KernelError::IntentStatusMismatch);
        }
        self.ensure_authorized(
            agent,
            capability,
            intent_record.resource,
            intent_record.kind.required_operation(),
        )?;
        if self.task_len >= TASKS {
            return Err(KernelError::TaskStoreFull);
        }
        self.ensure_event_slots(2)?;

        let task = TaskId::new(self.next_task);
        self.next_task += 1;
        self.tasks[self.task_len] = Task {
            id: task,
            intent,
            owner: agent,
            resource: intent_record.resource,
            assignee: None,
            delegated_capability: None,
            status: TaskStatus::Created,
        };
        self.task_len += 1;
        self.record_task_event(EventKind::TaskCreated, agent, Some(capability), task, None)?;
        self.set_intent_status(intent, IntentStatus::Bound)?;
        self.record_intent_task_event(IntentTaskEventKind::Bound, agent, task)?;
        Ok(task)
    }

    pub fn delegate_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        target_agent: AgentId,
    ) -> Result<Event, KernelError> {
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Delegate)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Act)?;
        ensure_status(current.status, &[TaskStatus::Created])?;
        self.ensure_event_slots(2)?;

        let delegated_capability = self.derive_task_capability(
            target_agent,
            current.resource,
            OperationSet::only(Operation::Act),
            task,
            capability,
        )?;
        let task_ref = self.find_task_mut(task)?;
        task_ref.assignee = Some(target_agent);
        task_ref.delegated_capability = Some(delegated_capability);
        task_ref.status = TaskStatus::Delegated;
        self.record_task_event(
            EventKind::DelegationRequested,
            agent,
            Some(delegated_capability),
            task,
            Some(target_agent),
        )
    }

    pub fn accept_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        let current = self.find_task(task)?;
        if current.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        ensure_status(current.status, &[TaskStatus::Delegated])?;
        self.ensure_event_slots(1)?;

        self.find_task_mut(task)?.status = TaskStatus::Accepted;
        self.record_task_event(EventKind::TaskAccepted, agent, None, task, None)
    }

    pub fn complete_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        let current = self.find_task(task)?;
        self.ensure_authorized_for_task(agent, capability, current.resource, Operation::Act, task)?;
        ensure_status(current.status, &[TaskStatus::Running])?;
        if current.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        self.ensure_event_slots(1)?;

        self.find_task_mut(task)?.status = TaskStatus::Completed;
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

fn ensure_status(current: TaskStatus, allowed: &[TaskStatus]) -> Result<(), KernelError> {
    allowed
        .contains(&current)
        .then_some(())
        .ok_or(KernelError::TaskStatusMismatch)
}
