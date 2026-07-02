//! Fixed-capacity task store and lifecycle transitions.
//!
//! This module belongs to `agent-kernel-core`. It owns task allocation,
//! lifecycle validation, capability-gated task mutation, and task event
//! recording. It performs no allocation or host I/O.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, Operation, ResourceId, Task,
    TaskId, TaskStatus,
};

impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize, const TASKS: usize>
    KernelCore<RESOURCES, CAPS, EVENTS, TASKS>
{
    pub fn create_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
    ) -> Result<TaskId, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        if self.task_len >= TASKS {
            return Err(KernelError::TaskStoreFull);
        }
        self.ensure_task_event_capacity()?;

        let task = TaskId::new(self.next_task);
        self.next_task += 1;
        self.tasks[self.task_len] = Task {
            id: task,
            owner: agent,
            resource,
            assignee: None,
            status: TaskStatus::Created,
        };
        self.task_len += 1;
        self.record_task_event(EventKind::TaskCreated, agent, Some(capability), task, None)?;
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
        ensure_status(current.status, &[TaskStatus::Created])?;
        self.ensure_task_event_capacity()?;

        let task_ref = self.find_task_mut(task)?;
        task_ref.assignee = Some(target_agent);
        task_ref.status = TaskStatus::Delegated;
        self.record_task_event(
            EventKind::DelegationRequested,
            agent,
            Some(capability),
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
        self.ensure_task_event_capacity()?;

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
        self.ensure_authorized(agent, capability, current.resource, Operation::Act)?;
        ensure_status(current.status, &[TaskStatus::Accepted])?;
        if current.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        self.ensure_task_event_capacity()?;

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
        self.ensure_task_event_capacity()?;

        self.find_task_mut(task)?.status = TaskStatus::Verified;
        self.record_task_event(EventKind::TaskVerified, agent, Some(capability), task, None)
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
                TaskStatus::Completed,
            ],
        )?;
        self.ensure_task_event_capacity()?;

        self.find_task_mut(task)?.status = TaskStatus::Cancelled;
        self.record_task_event(
            EventKind::TaskCancelled,
            agent,
            Some(capability),
            task,
            None,
        )
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

    fn record_task_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: Option<CapabilityId>,
        task: TaskId,
        target_agent: Option<AgentId>,
    ) -> Result<Event, KernelError> {
        let task_record = self.find_task(task)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind,
            resource: Some(task_record.resource),
            capability,
            action: None,
            operation: None,
            checkpoint: None,
            task: Some(task),
            target_agent,
        })
    }

    fn ensure_task_event_capacity(&self) -> Result<(), KernelError> {
        if self.event_len >= EVENTS {
            Err(KernelError::EventLogFull)
        } else {
            Ok(())
        }
    }
}

fn ensure_status(current: TaskStatus, allowed: &[TaskStatus]) -> Result<(), KernelError> {
    if allowed.iter().any(|status| *status == current) {
        Ok(())
    } else {
        Err(KernelError::TaskStatusMismatch)
    }
}
