//! Fixed-capacity task allocation, validation, mutation, and events.

use crate::{
    intent_event::IntentTaskEventKind, task_lookup::ensure_status, AgentId, CapabilityId, Event,
    EventKind, IntentId, IntentStatus, KernelCore, KernelError, Operation, OperationSet, Task,
    TaskId, TaskStatus,
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
    >
{
    pub fn create_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        intent: IntentId,
    ) -> Result<TaskId, KernelError> {
        self.ensure_agent_active(agent)?;
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
            run_ticks: 0,
            quantum_remaining: 0,
            last_fault: None,
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
        self.ensure_agent_active(agent)?;
        let current = self.find_task(task)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Delegate)?;
        self.ensure_authorized(agent, capability, current.resource, Operation::Act)?;
        ensure_status(current.status, &[TaskStatus::Created])?;
        self.ensure_agent_active(target_agent)?;
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
        self.ensure_agent_active(agent)?;
        let current = self.find_task(task)?;
        if current.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        ensure_status(current.status, &[TaskStatus::Delegated])?;
        self.ensure_event_slots(1)?;

        self.find_task_mut(task)?.status = TaskStatus::Accepted;
        self.record_task_event(EventKind::TaskAccepted, agent, None, task, None)
    }
}
