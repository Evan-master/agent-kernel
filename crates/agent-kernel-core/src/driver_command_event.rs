//! Fixed-capacity native driver command lifecycle.
//!
//! This module belongs to `agent-kernel-core`. It authorizes bound Driver
//! Agents, stores command transitions, validates optional device-event causes,
//! and records replayable events. It never executes hardware or host I/O.

use crate::{
    AgentId, CapabilityId, DeviceEventId, DeviceEventStatus, DriverBindingId, DriverCommandId,
    DriverCommandKind, DriverCommandPayload, DriverCommandRecord, DriverCommandResult,
    DriverCommandStatus, Event, EventKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceId, VerificationRequirement,
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
    pub fn submit_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        cause: Option<DeviceEventId>,
        kind: DriverCommandKind,
        payload: DriverCommandPayload,
    ) -> Result<DriverCommandId, KernelError> {
        self.ensure_agent_active(driver)?;
        let resource_record = self.find_resource(resource)?;
        Self::ensure_driver_resource(resource_record.kind)?;
        let binding = self.find_driver_binding_for_resource(resource)?;
        if binding.driver != driver {
            return Err(KernelError::AgentMismatch);
        }
        self.ensure_authorized(driver, capability, resource, Operation::Act)?;
        self.validate_driver_command_cause(binding.id, resource, cause)?;
        if self.driver_command_len >= DRIVER_COMMANDS {
            return Err(KernelError::DriverCommandStoreFull);
        }
        self.ensure_event_slots(1)?;

        let id = DriverCommandId::new(self.next_driver_command);
        self.next_driver_command += 1;
        self.driver_commands[self.driver_command_len] = DriverCommandRecord {
            id,
            binding: binding.id,
            resource,
            driver,
            cause,
            kind,
            payload,
            status: DriverCommandStatus::Submitted,
            result: None,
        };
        self.driver_command_len += 1;
        self.record_driver_command_event(
            EventKind::DriverCommandSubmitted,
            driver,
            capability,
            binding.id,
            id,
            resource,
            cause,
            kind,
            payload,
            None,
        )?;
        Ok(id)
    }

    pub fn complete_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        command: DriverCommandId,
        result: DriverCommandResult,
    ) -> Result<Event, KernelError> {
        self.transition_driver_command(
            driver,
            capability,
            command,
            DriverCommandStatus::Completed,
            EventKind::DriverCommandCompleted,
            result,
        )
    }

    pub fn fail_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        command: DriverCommandId,
        result: DriverCommandResult,
    ) -> Result<Event, KernelError> {
        self.transition_driver_command(
            driver,
            capability,
            command,
            DriverCommandStatus::Failed,
            EventKind::DriverCommandFailed,
            result,
        )
    }

    pub fn driver_commands(&self) -> &[DriverCommandRecord] {
        &self.driver_commands[..self.driver_command_len]
    }

    fn transition_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        command: DriverCommandId,
        to: DriverCommandStatus,
        event_kind: EventKind,
        result: DriverCommandResult,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(driver)?;
        let record = self.find_driver_command(command)?;
        if record.status != DriverCommandStatus::Submitted {
            return Err(KernelError::DriverCommandStatusMismatch);
        }
        let binding = self.find_driver_binding(record.binding)?;
        if binding.driver != driver || record.driver != driver {
            return Err(KernelError::AgentMismatch);
        }
        self.find_resource(record.resource)?;
        self.ensure_authorized(driver, capability, record.resource, Operation::Act)?;
        self.ensure_event_slots(1)?;

        let stored = self.find_driver_command_mut(command)?;
        stored.status = to;
        stored.result = Some(result);
        self.record_driver_command_event(
            event_kind,
            driver,
            capability,
            record.binding,
            command,
            record.resource,
            record.cause,
            record.kind,
            record.payload,
            Some(result),
        )
    }

    fn validate_driver_command_cause(
        &self,
        binding: DriverBindingId,
        resource: ResourceId,
        cause: Option<DeviceEventId>,
    ) -> Result<(), KernelError> {
        let Some(cause) = cause else {
            return Ok(());
        };
        let event = self.find_device_event(cause)?;
        if event.binding != binding || event.resource != resource {
            return Err(KernelError::DriverCommandCauseMismatch);
        }
        if event.status == DeviceEventStatus::Raised {
            return Err(KernelError::DeviceEventStatusMismatch);
        }
        Ok(())
    }

    fn find_driver_command(&self, id: DriverCommandId) -> Result<DriverCommandRecord, KernelError> {
        self.driver_commands()
            .iter()
            .find(|command| command.id == id)
            .copied()
            .ok_or(KernelError::DriverCommandNotFound)
    }

    fn find_driver_command_mut(
        &mut self,
        id: DriverCommandId,
    ) -> Result<&mut DriverCommandRecord, KernelError> {
        self.driver_commands[..self.driver_command_len]
            .iter_mut()
            .find(|command| command.id == id)
            .ok_or(KernelError::DriverCommandNotFound)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_driver_command_event(
        &mut self,
        event_kind: EventKind,
        driver: AgentId,
        capability: CapabilityId,
        binding: DriverBindingId,
        command: DriverCommandId,
        resource: ResourceId,
        cause: Option<DeviceEventId>,
        command_kind: DriverCommandKind,
        payload: DriverCommandPayload,
        result: Option<DriverCommandResult>,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent: driver,
            kind: event_kind,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
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
            task: None,
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
            driver_binding: Some(binding),
            device_event: cause,
            device_event_kind: None,
            device_event_payload: None,
            driver_command: Some(command),
            driver_command_kind: Some(command_kind),
            driver_command_payload: Some(payload),
            driver_command_result: result,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
