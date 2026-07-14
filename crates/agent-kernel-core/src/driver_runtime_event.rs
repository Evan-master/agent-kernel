//! Device-event delivery into Driver Agent runtime work.
//!
//! This core module atomically turns a raised device event into a queued
//! `DriverInvocation` and gates acknowledgement on a running invocation. It
//! performs no hardware I/O and leaves every failed transition unmodified.

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
    pub fn deliver_device_event(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        event: DeviceEventId,
    ) -> Result<DriverInvocationId, KernelError> {
        self.ensure_agent_active(driver)?;
        let record = self.find_device_event(event)?;
        if record.status != DeviceEventStatus::Raised {
            return Err(KernelError::DeviceEventStatusMismatch);
        }
        let binding = self.find_driver_binding(record.binding)?;
        if binding.driver != driver {
            return Err(KernelError::AgentMismatch);
        }
        self.find_resource(record.resource)?;
        self.ensure_authorized(driver, capability, record.resource, Operation::Observe)?;
        self.ensure_agent_admitted_for_driver(driver, record.binding, record.resource)?;
        if self.driver_invocation_len >= DRIVER_INVOCATIONS {
            return Err(KernelError::DriverInvocationStoreFull);
        }
        self.ensure_event_slots(2)?;

        let invocation = DriverInvocationId::new(self.next_driver_invocation);
        self.next_driver_invocation += 1;
        self.driver_invocations[self.driver_invocation_len] = DriverInvocationRecord {
            id: invocation,
            binding: record.binding,
            driver,
            resource: record.resource,
            event,
            status: DriverInvocationStatus::Queued,
            run_ticks: 0,
            quantum_remaining: 0,
        };
        self.driver_invocation_len += 1;
        self.find_device_event_mut(event)?.status = DeviceEventStatus::Delivered;
        self.record_device_event(
            EventKind::DeviceEventDelivered,
            driver,
            capability,
            record.binding,
            event,
            record.resource,
            record.kind,
            record.payload,
            Some(invocation),
        )?;
        self.record_driver_invocation_event(
            EventKind::DriverInvocationQueued,
            driver,
            Some(capability),
            invocation,
            Operation::Observe,
            None,
            None,
        )?;
        Ok(invocation)
    }

    pub fn acknowledge_device_event(
        &mut self,
        driver: AgentId,
        capability: CapabilityId,
        event: DeviceEventId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(driver)?;
        let record = self.find_device_event(event)?;
        if record.status != DeviceEventStatus::Delivered {
            return Err(KernelError::DeviceEventStatusMismatch);
        }
        let binding = self.find_driver_binding(record.binding)?;
        if binding.driver != driver {
            return Err(KernelError::AgentMismatch);
        }
        self.find_resource(record.resource)?;
        self.ensure_authorized(driver, capability, record.resource, Operation::Act)?;
        let invocation = self.find_driver_invocation_for_event(event)?;
        if invocation.status != DriverInvocationStatus::Running || invocation.driver != driver {
            return Err(KernelError::DriverInvocationNotRunnable);
        }
        self.ensure_execution_context_running_driver(driver, invocation.id)?;
        self.ensure_event_slots(1)?;

        self.find_device_event_mut(event)?.status = DeviceEventStatus::Acknowledged;
        self.record_device_event(
            EventKind::DeviceEventAcknowledged,
            driver,
            capability,
            record.binding,
            event,
            record.resource,
            record.kind,
            record.payload,
            Some(invocation.id),
        )
    }
}
