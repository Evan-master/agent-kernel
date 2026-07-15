//! Fixed-capacity agent launch entry store.
//!
//! This module owns the native launch transition that binds a registered agent
//! to a resource, capability, and optional declared action intent.

use crate::{
    AgentEntryKind, AgentEntryRecord, AgentId, AgentImageId, CapabilityId, Event, EventKind,
    IntentId, IntentKind, IntentStatus, KernelCore, KernelError, Operation, OperationSet,
    ResourceId, TaskId, TaskStatus, VerificationRequirement,
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
    pub fn launch_agent(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        image: AgentImageId,
        kind: AgentEntryKind,
        intent: Option<IntentId>,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        if self.find_agent_entry(agent).is_ok() {
            return Err(KernelError::AgentAlreadyLaunched);
        }
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        if let Some(intent_id) = intent {
            self.ensure_launch_intent(agent, resource, intent_id)?;
        }
        self.ensure_launch_image(image, resource, kind)?;
        if self.agent_entry_len >= AGENTS {
            return Err(KernelError::AgentEntryStoreFull);
        }
        self.ensure_event_slots(1)?;

        self.agent_entries[self.agent_entry_len] = AgentEntryRecord {
            agent,
            resource,
            capability,
            image,
            kind,
            intent,
            task: None,
        };
        self.agent_entry_len += 1;
        self.record_agent_launch_event(agent, capability, resource, image, intent, None)
    }

    pub fn launch_task_agent(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        image: AgentImageId,
        kind: AgentEntryKind,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        if self.find_agent_entry(agent).is_ok() {
            return Err(KernelError::AgentAlreadyLaunched);
        }
        let task_record = self.find_task(task)?;
        if task_record.assignee != Some(agent) {
            return Err(KernelError::TaskAgentMismatch);
        }
        match task_record.status {
            TaskStatus::Delegated | TaskStatus::Accepted => {}
            _ => return Err(KernelError::TaskStatusMismatch),
        }
        self.ensure_authorized_for_task(
            agent,
            capability,
            task_record.resource,
            Operation::Act,
            task,
        )?;
        self.ensure_launch_image(image, task_record.resource, kind)?;
        if self.agent_entry_len >= AGENTS {
            return Err(KernelError::AgentEntryStoreFull);
        }
        self.ensure_event_slots(1)?;

        self.agent_entries[self.agent_entry_len] = AgentEntryRecord {
            agent,
            resource: task_record.resource,
            capability,
            image,
            kind,
            intent: Some(task_record.intent),
            task: Some(task),
        };
        self.agent_entry_len += 1;
        self.record_agent_launch_event(
            agent,
            capability,
            task_record.resource,
            image,
            Some(task_record.intent),
            Some(task),
        )
    }

    pub fn agent_entries(&self) -> &[AgentEntryRecord] {
        &self.agent_entries[..self.agent_entry_len]
    }

    pub fn agent_entry(&self, agent: AgentId) -> Result<AgentEntryRecord, KernelError> {
        self.find_agent_entry(agent)
    }

    pub(crate) fn find_agent_entry(&self, agent: AgentId) -> Result<AgentEntryRecord, KernelError> {
        self.agent_entries()
            .iter()
            .find(|entry| entry.agent == agent)
            .copied()
            .ok_or(KernelError::AgentEntryNotFound)
    }

    fn ensure_launch_intent(
        &self,
        agent: AgentId,
        resource: ResourceId,
        intent: IntentId,
    ) -> Result<(), KernelError> {
        let intent_record = self.find_intent(intent)?;
        if intent_record.owner != agent {
            return Err(KernelError::IntentAgentMismatch);
        }
        if intent_record.resource != resource {
            return Err(KernelError::ResourceMismatch);
        }
        if intent_record.kind != IntentKind::Act {
            return Err(KernelError::IntentKindMismatch);
        }
        if intent_record.status != IntentStatus::Declared {
            return Err(KernelError::IntentStatusMismatch);
        }

        Ok(())
    }

    fn record_agent_launch_event(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        image: AgentImageId,
        intent: Option<IntentId>,
        task: Option<TaskId>,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::AgentLaunched,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent,
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(Operation::Act),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task,
            task_result: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            signal: None,
            target_agent: Some(agent),
            driver_binding: None,
            device_event: None,
            device_event_kind: None,
            device_event_payload: None,
            driver_command: None,
            driver_command_kind: None,
            driver_command_payload: None,
            driver_command_result: None,
            driver_invocation: None,
            driver_invocation_ticks: None,
            driver_invocation_quantum: None,
            agent_image: Some(image),
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
