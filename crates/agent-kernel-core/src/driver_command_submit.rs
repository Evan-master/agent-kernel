//! Native Driver Command submission and causal validation.
//!
//! This core-layer module authorizes a bound Driver Agent, validates optional
//! Device Event causality, allocates fixed-capacity command records, and emits
//! submission events. Dispatch and terminal transitions live separately.

use crate::{
    AgentId, CapabilityId, DeviceEventId, DeviceEventStatus, DriverBindingId, DriverCommandId,
    DriverCommandKind, DriverCommandPayload, DriverCommandRecord, DriverCommandStatus,
    DriverInvocationId, DriverInvocationStatus, EventKind, KernelCore, KernelError, Operation,
    ResourceId,
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
        let invocation = self.validate_driver_command_cause(driver, binding.id, resource, cause)?;
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
            invocation,
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
            invocation,
            kind,
            payload,
            None,
        )?;
        Ok(id)
    }

    fn validate_driver_command_cause(
        &self,
        driver: AgentId,
        binding: DriverBindingId,
        resource: ResourceId,
        cause: Option<DeviceEventId>,
    ) -> Result<Option<DriverInvocationId>, KernelError> {
        let Some(cause) = cause else {
            return Ok(None);
        };
        let event = self.find_device_event(cause)?;
        if event.binding != binding || event.resource != resource {
            return Err(KernelError::DriverCommandCauseMismatch);
        }
        if event.status == DeviceEventStatus::Raised {
            return Err(KernelError::DeviceEventStatusMismatch);
        }
        let invocation = self.find_driver_invocation_for_event(cause)?;
        if invocation.driver != driver || invocation.status != DriverInvocationStatus::Running {
            return Err(KernelError::DriverInvocationNotRunnable);
        }
        self.ensure_execution_context_running_driver(driver, invocation.id)?;
        Ok(Some(invocation.id))
    }
}
