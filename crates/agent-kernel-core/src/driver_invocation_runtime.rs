//! Driver Invocation queue selection, dispatch, and completion.
//!
//! This module belongs to `agent-kernel-core`. It treats append order as a
//! deterministic per-driver FIFO queue, owns execution-context transitions,
//! and records every successful runtime mutation without host dependencies.

use crate::{
    AgentId, CapabilityId, DeviceEventId, DeviceEventStatus, DriverInvocationId,
    DriverInvocationRecord, DriverInvocationStatus, Event, EventKind, KernelCore, KernelError,
    Operation,
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
    pub fn dispatch_next_driver_invocation(
        &mut self,
        driver: AgentId,
        quantum: u64,
    ) -> Result<DriverInvocationId, KernelError> {
        if quantum == 0 {
            return Err(KernelError::DriverInvocationQuantumInvalid);
        }
        self.ensure_agent_active(driver)?;
        let invocation = self
            .driver_invocations()
            .iter()
            .find(|record| {
                record.driver == driver && record.status == DriverInvocationStatus::Queued
            })
            .copied()
            .ok_or(KernelError::DriverInvocationQueueEmpty)?;
        self.find_resource(invocation.resource)?;
        self.ensure_agent_admitted_for_driver(driver, invocation.binding, invocation.resource)?;
        self.ensure_execution_context_idle(driver)?;
        self.ensure_event_slots(1)?;

        let stored = self.find_driver_invocation_mut(invocation.id)?;
        stored.status = DriverInvocationStatus::Running;
        stored.quantum_remaining = quantum;
        self.set_execution_context_running_driver(
            driver,
            invocation.id,
            invocation.run_ticks,
            quantum,
        )?;
        self.record_driver_invocation_event(
            EventKind::DriverInvocationDispatched,
            driver,
            None,
            invocation.id,
            Operation::Act,
            Some(invocation.run_ticks),
            Some(quantum),
        )?;
        Ok(invocation.id)
    }

    pub fn complete_driver_invocation(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        invocation: DriverInvocationId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(driver)?;
        let record = self.find_driver_invocation(invocation)?;
        if record.status != DriverInvocationStatus::Running {
            return Err(KernelError::DriverInvocationStatusMismatch);
        }
        if record.driver != driver {
            return Err(KernelError::AgentMismatch);
        }
        self.find_resource(record.resource)?;
        self.ensure_agent_admitted_for_driver(driver, record.binding, record.resource)?;
        self.ensure_execution_context_running_driver(driver, invocation)?;
        self.ensure_authorized(driver, capability, record.resource, Operation::Act)?;
        if self.find_device_event(record.event)?.status != DeviceEventStatus::Acknowledged {
            return Err(KernelError::DeviceEventStatusMismatch);
        }
        self.ensure_event_slots(1)?;

        let stored = self.find_driver_invocation_mut(invocation)?;
        stored.status = DriverInvocationStatus::Completed;
        stored.quantum_remaining = 0;
        self.set_execution_context_idle(driver)?;
        self.record_driver_invocation_event(
            EventKind::DriverInvocationCompleted,
            driver,
            Some(capability),
            invocation,
            Operation::Act,
            Some(record.run_ticks),
            Some(0),
        )
    }

    pub fn driver_invocations(&self) -> &[DriverInvocationRecord] {
        &self.driver_invocations[..self.driver_invocation_len]
    }

    pub(crate) fn find_driver_invocation(
        &self,
        id: DriverInvocationId,
    ) -> Result<DriverInvocationRecord, KernelError> {
        self.driver_invocations()
            .iter()
            .find(|record| record.id == id)
            .copied()
            .ok_or(KernelError::DriverInvocationNotFound)
    }

    pub(crate) fn find_driver_invocation_for_event(
        &self,
        event: DeviceEventId,
    ) -> Result<DriverInvocationRecord, KernelError> {
        self.driver_invocations()
            .iter()
            .find(|record| record.event == event)
            .copied()
            .ok_or(KernelError::DriverInvocationNotFound)
    }

    pub(crate) fn find_driver_invocation_mut(
        &mut self,
        id: DriverInvocationId,
    ) -> Result<&mut DriverInvocationRecord, KernelError> {
        self.driver_invocations[..self.driver_invocation_len]
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or(KernelError::DriverInvocationNotFound)
    }
}
