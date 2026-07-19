//! Fixed-capacity kernel action store behavior.
//!
//! This module records authorized actions, tracks verification requests, and
//! emits replayable events without allocation.

use crate::{
    ActionId, ActionRecord, ActionStatus, AgentId, CapabilityId, Event, EventKind, KernelCore,
    KernelError, Operation, OperationSet, ResourceId, VerificationRequirement,
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
    pub fn act(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        if self.find_action(action).is_ok() {
            return Err(KernelError::ActionAlreadyExists);
        }
        if self.action_len >= ACTIONS {
            return Err(KernelError::ActionStoreFull);
        }
        self.ensure_event_slots(1)?;

        self.actions[self.action_len] = ActionRecord {
            id: action,
            agent,
            resource,
            capability,
            status: ActionStatus::Executed,
        };
        self.action_len += 1;

        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::ActionExecuted,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: Some(action),
            observation: None,
            message: None,
            message_kind: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(Operation::Act),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            runtime_admission: None,
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
            target_agent: None,
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
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }

    pub fn verify(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Verify)?;

        let record = self.find_action(action)?;
        if record.resource != resource {
            return Err(KernelError::ActionResourceMismatch);
        }
        if record.status != ActionStatus::Executed {
            return Err(KernelError::ActionStatusMismatch);
        }
        self.ensure_event_slots(1)?;

        self.find_action_mut(action)?.status = ActionStatus::VerificationRequested;

        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::VerificationRequested,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: Some(action),
            observation: None,
            message: None,
            message_kind: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(Operation::Verify),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            runtime_admission: None,
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
            target_agent: None,
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
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }

    pub fn actions(&self) -> &[ActionRecord] {
        &self.actions[..self.action_len]
    }

    pub(crate) fn find_action(&self, id: ActionId) -> Result<ActionRecord, KernelError> {
        for action in self.actions() {
            if action.id == id {
                return Ok(*action);
            }
        }

        Err(KernelError::ActionNotFound)
    }

    pub(crate) fn find_action_mut(
        &mut self,
        id: ActionId,
    ) -> Result<&mut ActionRecord, KernelError> {
        for action in &mut self.actions[..self.action_len] {
            if action.id == id {
                return Ok(action);
            }
        }

        Err(KernelError::ActionNotFound)
    }
}
