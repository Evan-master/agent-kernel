//! Fixed-capacity device event lifecycle.
//!
//! This module belongs to `agent-kernel-core`. It records external events for
//! device-like resources and moves them through driver delivery and
//! acknowledgement under explicit capabilities. It never performs host I/O,
//! parses byte streams, or executes a driver.

use crate::{
    AgentId, CapabilityId, DeviceEventId, DriverBindingId, DriverInvocationId, Event, EventKind,
    KernelCore, KernelError, Operation, OperationSet, ResourceId, VerificationRequirement,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DeviceEventKind {
    Interrupt,
    DataReady,
    Fault,
    StateChanged,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DeviceEventPayload {
    pub code: u16,
    pub value: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DeviceEventStatus {
    Raised,
    Delivered,
    Acknowledged,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DeviceEventRecord {
    pub id: DeviceEventId,
    pub binding: DriverBindingId,
    pub resource: ResourceId,
    pub kind: DeviceEventKind,
    pub payload: DeviceEventPayload,
    pub status: DeviceEventStatus,
}

impl DeviceEventRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: DeviceEventId::new(0),
            binding: DriverBindingId::new(0),
            resource: ResourceId::new(0),
            kind: DeviceEventKind::Interrupt,
            payload: DeviceEventPayload { code: 0, value: 0 },
            status: DeviceEventStatus::Acknowledged,
        }
    }
}

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
    pub fn raise_device_event(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: DeviceEventKind,
        payload: DeviceEventPayload,
    ) -> Result<DeviceEventId, KernelError> {
        self.ensure_agent_active(agent)?;
        let resource_record = self.find_resource(resource)?;
        Self::ensure_driver_resource(resource_record.kind)?;
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        let binding = self.find_driver_binding_for_resource(resource)?;
        if self.device_event_len >= DEVICE_EVENTS {
            return Err(KernelError::DeviceEventStoreFull);
        }
        self.ensure_event_slots(1)?;

        let id = DeviceEventId::new(self.next_device_event);
        self.next_device_event += 1;
        self.device_events[self.device_event_len] = DeviceEventRecord {
            id,
            binding: binding.id,
            resource,
            kind,
            payload,
            status: DeviceEventStatus::Raised,
        };
        self.device_event_len += 1;
        self.record_device_event(
            EventKind::DeviceEventRaised,
            agent,
            capability,
            binding.id,
            id,
            resource,
            kind,
            payload,
            None,
        )?;
        Ok(id)
    }

    pub fn device_events(&self) -> &[DeviceEventRecord] {
        &self.device_events[..self.device_event_len]
    }

    pub(crate) fn find_device_event(
        &self,
        id: DeviceEventId,
    ) -> Result<DeviceEventRecord, KernelError> {
        self.device_events()
            .iter()
            .find(|event| event.id == id)
            .copied()
            .ok_or(KernelError::DeviceEventNotFound)
    }

    pub(crate) fn find_device_event_mut(
        &mut self,
        id: DeviceEventId,
    ) -> Result<&mut DeviceEventRecord, KernelError> {
        for event in &mut self.device_events[..self.device_event_len] {
            if event.id == id {
                return Ok(event);
            }
        }

        Err(KernelError::DeviceEventNotFound)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_device_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: CapabilityId,
        binding: DriverBindingId,
        event: DeviceEventId,
        resource: ResourceId,
        event_kind: DeviceEventKind,
        payload: DeviceEventPayload,
        invocation: Option<DriverInvocationId>,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind,
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
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
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
            driver_binding: Some(binding),
            device_event: Some(event),
            device_event_kind: Some(event_kind),
            device_event_payload: Some(payload),
            driver_command: None,
            driver_command_kind: None,
            driver_command_payload: None,
            driver_command_result: None,
            driver_invocation: invocation,
            driver_invocation_ticks: None,
            driver_invocation_quantum: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
