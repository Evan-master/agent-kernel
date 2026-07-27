//! Bounded Driver Invocation fault containment and owner recovery.
//!
//! A native trap moves only the active Invocation into `Faulted`. Recovery
//! requires resource rollback authority and is allowed once, before any
//! acknowledged event or Driver Command side effect.

use crate::{
    AgentId, CapabilityId, DeviceEventStatus, DriverInvocationId, DriverInvocationStatus, Event,
    EventKind, FaultKind, KernelCore, KernelError, Operation,
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
    pub fn fault_driver_invocation(
        &mut self,
        driver: AgentId,
        invocation: DriverInvocationId,
        kind: FaultKind,
        detail: u64,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(driver)?;
        let record = self.find_driver_invocation(invocation)?;
        if record.driver != driver {
            return Err(KernelError::AgentMismatch);
        }
        if record.status != DriverInvocationStatus::Running {
            return Err(KernelError::DriverInvocationNotRunnable);
        }
        self.find_resource(record.resource)?;
        self.ensure_agent_admitted_for_driver(driver, record.binding, record.resource)?;
        self.ensure_execution_context_running_driver(driver, invocation)?;
        self.ensure_event_slots(1)?;

        let stored = self.find_driver_invocation_mut(invocation)?;
        stored.status = DriverInvocationStatus::Faulted;
        stored.quantum_remaining = 0;
        stored.fault_kind = Some(kind);
        stored.fault_detail = Some(detail);
        self.set_execution_context_faulted_driver(driver, invocation)?;
        self.record_driver_invocation_fault_event(
            EventKind::DriverInvocationFaulted,
            driver,
            None,
            None,
            invocation,
            None,
            kind,
            detail,
        )
    }

    pub fn recover_driver_invocation(
        &mut self,
        actor: AgentId,
        capability: CapabilityId,
        driver: AgentId,
        invocation: DriverInvocationId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(actor)?;
        self.ensure_agent_active(driver)?;
        let record = self.find_driver_invocation(invocation)?;
        if record.driver != driver {
            return Err(KernelError::AgentMismatch);
        }
        if record.status != DriverInvocationStatus::Faulted {
            return Err(KernelError::DriverInvocationStatusMismatch);
        }
        self.find_resource(record.resource)?;
        self.ensure_execution_context_faulted_driver(driver, invocation)?;
        self.ensure_authorized(actor, capability, record.resource, Operation::Rollback)?;
        if record.restart_generation != 0 {
            return Err(KernelError::DriverInvocationRestartLimitReached);
        }
        if self.find_device_event(record.event)?.status != DeviceEventStatus::Delivered
            || self
                .driver_commands()
                .iter()
                .any(|command| command.invocation == Some(invocation))
        {
            return Err(KernelError::DriverInvocationRecoveryUnsafe);
        }
        let kind = record
            .fault_kind
            .ok_or(KernelError::DriverInvocationStatusMismatch)?;
        let detail = record
            .fault_detail
            .ok_or(KernelError::DriverInvocationStatusMismatch)?;
        self.ensure_event_slots(1)?;

        let stored = self.find_driver_invocation_mut(invocation)?;
        stored.status = DriverInvocationStatus::Queued;
        stored.quantum_remaining = 0;
        stored.restart_generation = 1;
        self.set_execution_context_idle(driver)?;
        self.record_driver_invocation_fault_event(
            EventKind::DriverInvocationRecovered,
            actor,
            Some(capability),
            Some(driver),
            invocation,
            Some(Operation::Rollback),
            kind,
            detail,
        )
    }
}
