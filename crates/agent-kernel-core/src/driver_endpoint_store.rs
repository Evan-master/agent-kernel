//! Authorized fixed-capacity Driver Endpoint registration and lookup.
//!
//! This core-layer module validates device resources, delegated authority,
//! descriptor ranges, and address-space overlap before storing one immutable
//! endpoint per resource. It allocates nothing and performs no endpoint I/O.

use crate::{
    AgentId, CapabilityId, DriverEndpointDescriptor, DriverEndpointKind, DriverEndpointRecord,
    Event, KernelCore, KernelError, Operation, ResourceId,
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
    pub fn register_driver_endpoint(
        &mut self,
        installer: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        descriptor: DriverEndpointDescriptor,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(installer)?;
        let resource_record = self.find_resource(resource)?;
        Self::ensure_driver_resource(resource_record.kind)?;
        self.ensure_authorized(installer, capability, resource, Operation::Delegate)?;
        if self.find_driver_endpoint_record(resource).is_ok() {
            return Err(KernelError::DriverEndpointAlreadyExists);
        }
        self.validate_driver_endpoint_descriptor(descriptor)?;
        self.ensure_driver_endpoint_does_not_overlap(descriptor)?;
        if self.driver_endpoint_len >= RESOURCES {
            return Err(KernelError::DriverEndpointStoreFull);
        }
        self.ensure_event_slots(1)?;

        self.driver_endpoints[self.driver_endpoint_len] = DriverEndpointRecord {
            resource,
            installer,
            descriptor,
        };
        self.driver_endpoint_len += 1;
        self.record_driver_endpoint_registered(installer, capability, resource)
    }

    pub fn driver_endpoints(&self) -> &[DriverEndpointRecord] {
        &self.driver_endpoints[..self.driver_endpoint_len]
    }

    pub fn driver_endpoint(
        &self,
        resource: ResourceId,
    ) -> Result<DriverEndpointRecord, KernelError> {
        self.find_resource(resource)?;
        self.find_driver_endpoint_record(resource)
    }

    pub(crate) fn find_driver_endpoint_record(
        &self,
        resource: ResourceId,
    ) -> Result<DriverEndpointRecord, KernelError> {
        self.driver_endpoints()
            .iter()
            .find(|endpoint| endpoint.resource == resource)
            .copied()
            .ok_or(KernelError::DriverEndpointNotFound)
    }

    fn validate_driver_endpoint_descriptor(
        &self,
        descriptor: DriverEndpointDescriptor,
    ) -> Result<(), KernelError> {
        let end = descriptor
            .end()
            .ok_or(KernelError::DriverEndpointDescriptorInvalid)?;
        if descriptor.kind == DriverEndpointKind::Port && end > u16::MAX as u64 {
            return Err(KernelError::DriverEndpointDescriptorInvalid);
        }
        Ok(())
    }

    fn ensure_driver_endpoint_does_not_overlap(
        &self,
        candidate: DriverEndpointDescriptor,
    ) -> Result<(), KernelError> {
        let candidate_end = candidate
            .end()
            .ok_or(KernelError::DriverEndpointDescriptorInvalid)?;
        for endpoint in self.driver_endpoints() {
            let existing = endpoint.descriptor;
            if existing.kind != candidate.kind {
                continue;
            }
            let existing_end = existing
                .end()
                .ok_or(KernelError::DriverEndpointDescriptorInvalid)?;
            if candidate.base <= existing_end && existing.base <= candidate_end {
                return Err(KernelError::DriverEndpointOverlap);
            }
        }
        Ok(())
    }
}
