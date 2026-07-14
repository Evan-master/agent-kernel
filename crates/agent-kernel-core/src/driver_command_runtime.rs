//! Native Driver Command dispatch and terminal transitions.
//!
//! This core-layer module authorizes dispatch, returns immutable HAL requests,
//! and records backend outcomes without performing device I/O.

use crate::{
    AgentId, CapabilityId, DriverCommandId, DriverCommandRecord, DriverCommandRequest,
    DriverCommandResult, DriverCommandStatus, DriverInvocationStatus, Event, EventKind, KernelCore,
    KernelError, Operation,
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
    pub fn dispatch_driver_command(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        command: DriverCommandId,
    ) -> Result<DriverCommandRequest, KernelError> {
        self.ensure_agent_active(driver)?;
        let record = self.find_driver_command(command)?;
        if record.status != DriverCommandStatus::Submitted {
            return Err(KernelError::DriverCommandStatusMismatch);
        }
        let binding = self.find_driver_binding(record.binding)?;
        if binding.driver != driver || record.driver != driver {
            return Err(KernelError::AgentMismatch);
        }
        let resource = self.find_resource(record.resource)?;
        Self::ensure_driver_resource(resource.kind)?;
        self.ensure_authorized(driver, capability, record.resource, Operation::Act)?;
        self.ensure_driver_command_invocation_running(record)?;
        self.ensure_event_slots(1)?;

        self.find_driver_command_mut(command)?.status = DriverCommandStatus::Dispatched;
        self.record_driver_command_event(
            EventKind::DriverCommandDispatched,
            driver,
            capability,
            record.binding,
            command,
            record.resource,
            record.cause,
            record.invocation,
            record.kind,
            record.payload,
            None,
        )?;

        Ok(DriverCommandRequest {
            command,
            binding: record.binding,
            resource: record.resource,
            driver,
            cause: record.cause,
            invocation: record.invocation,
            kind: record.kind,
            payload: record.payload,
        })
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
        if record.status != DriverCommandStatus::Dispatched {
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
            record.invocation,
            record.kind,
            record.payload,
            Some(result),
        )
    }

    fn ensure_driver_command_invocation_running(
        &self,
        record: DriverCommandRecord,
    ) -> Result<(), KernelError> {
        let Some(invocation_id) = record.invocation else {
            return Ok(());
        };
        let invocation = self.find_driver_invocation(invocation_id)?;
        if invocation.driver != record.driver
            || invocation.status != DriverInvocationStatus::Running
        {
            return Err(KernelError::DriverInvocationNotRunnable);
        }
        self.ensure_execution_context_running_driver(record.driver, invocation_id)
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
}
