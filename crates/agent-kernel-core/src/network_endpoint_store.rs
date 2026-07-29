//! Fixed-capacity network endpoint authority and lifecycle.
//!
//! Endpoint Resources are Device children. Core records policy transitions;
//! architecture code owns hardware activation and quiescence between phases.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, NetworkEndpointConfig,
    NetworkEndpointRecord, NetworkEndpointStatus, NetworkTransferDirection, NetworkTransferStatus,
    Operation, OperationSet, ResourceCreateOutcome, ResourceId, ResourceKind,
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
    pub fn create_network_endpoint(
        &mut self,
        agent: AgentId,
        device_capability: CapabilityId,
        device: ResourceId,
        config: NetworkEndpointConfig,
        operations: OperationSet,
    ) -> Result<ResourceCreateOutcome, KernelError> {
        self.ensure_agent_active(agent)?;
        if self.find_resource(device)?.kind != ResourceKind::Device {
            return Err(KernelError::ResourceKindMismatch);
        }
        self.ensure_authorized(agent, device_capability, device, Operation::Act)?;
        if self
            .network_endpoints()
            .iter()
            .any(|endpoint| endpoint.device() == device && endpoint.occupies_endpoint())
        {
            return Err(KernelError::NetworkEndpointAlreadyExists);
        }
        if self.network_endpoint_len >= RESOURCES {
            return Err(KernelError::NetworkEndpointStoreFull);
        }
        self.ensure_event_slots(3)?;

        let outcome = self.create_resource(
            agent,
            ResourceKind::Network,
            Some((device, device_capability)),
            operations,
        )?;
        self.network_endpoints[self.network_endpoint_len] =
            NetworkEndpointRecord::new(outcome.resource, device, config);
        self.network_endpoint_len += 1;
        self.record_network_event(
            EventKind::NetworkEndpointReserved,
            agent,
            outcome.capability,
            outcome.resource,
            Operation::Act,
        )?;
        Ok(outcome)
    }

    pub fn activate_network_endpoint(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
    ) -> Result<Event, KernelError> {
        self.transition_network_endpoint(
            agent,
            capability,
            endpoint,
            NetworkEndpointStatus::Reserved,
            NetworkEndpointStatus::Active,
            EventKind::NetworkEndpointActivated,
            Operation::Act,
        )
    }

    pub fn begin_network_endpoint_revoke(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let record = self.network_endpoint(endpoint)?;
        self.ensure_authorized(agent, capability, endpoint, Operation::Rollback)?;
        if record.status() != NetworkEndpointStatus::Active {
            return Err(KernelError::NetworkEndpointStatusMismatch);
        }
        if self.network_transfers().iter().any(|transfer| {
            transfer.endpoint() == endpoint
                && transfer.direction() == NetworkTransferDirection::Transmit
                && transfer.status() == NetworkTransferStatus::Prepared
        }) {
            return Err(KernelError::NetworkTransferPending);
        }
        self.ensure_event_slots(1)?;
        self.set_network_endpoint_status(endpoint, NetworkEndpointStatus::Revoking)?;
        self.record_network_event(
            EventKind::NetworkEndpointRevoking,
            agent,
            capability,
            endpoint,
            Operation::Rollback,
        )
    }

    pub fn complete_network_endpoint_revoke(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
    ) -> Result<Event, KernelError> {
        self.transition_network_endpoint(
            agent,
            capability,
            endpoint,
            NetworkEndpointStatus::Revoking,
            NetworkEndpointStatus::Released,
            EventKind::NetworkEndpointReleased,
            Operation::Rollback,
        )
    }

    pub fn network_endpoints(&self) -> &[NetworkEndpointRecord] {
        &self.network_endpoints[..self.network_endpoint_len]
    }

    pub fn network_endpoint(
        &self,
        resource: ResourceId,
    ) -> Result<NetworkEndpointRecord, KernelError> {
        self.network_endpoints()
            .iter()
            .find(|endpoint| endpoint.resource() == resource)
            .copied()
            .ok_or(KernelError::NetworkEndpointNotFound)
    }

    pub(crate) fn ensure_network_resource_quiescent(
        &self,
        resource: ResourceId,
    ) -> Result<(), KernelError> {
        if self.network_endpoints().iter().any(|endpoint| {
            endpoint.occupies_endpoint()
                && (endpoint.resource() == resource || endpoint.device() == resource)
        }) {
            Err(KernelError::NetworkResourceBusy)
        } else {
            Ok(())
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn transition_network_endpoint(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        expected: NetworkEndpointStatus,
        next: NetworkEndpointStatus,
        event_kind: EventKind,
        operation: Operation,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let record = self.network_endpoint(endpoint)?;
        self.ensure_authorized(agent, capability, endpoint, operation)?;
        if record.status() != expected {
            return Err(KernelError::NetworkEndpointStatusMismatch);
        }
        self.ensure_event_slots(1)?;
        self.set_network_endpoint_status(endpoint, next)?;
        self.record_network_event(event_kind, agent, capability, endpoint, operation)
    }

    fn set_network_endpoint_status(
        &mut self,
        endpoint: ResourceId,
        status: NetworkEndpointStatus,
    ) -> Result<(), KernelError> {
        let slot = self.network_endpoints[..self.network_endpoint_len]
            .iter_mut()
            .find(|record| record.resource() == endpoint)
            .ok_or(KernelError::NetworkEndpointNotFound)?;
        slot.set_status(status);
        Ok(())
    }

    pub(crate) fn record_network_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        operation: Operation,
    ) -> Result<Event, KernelError> {
        let mut event = Event::empty();
        event.agent = agent;
        event.kind = kind;
        event.resource = Some(resource);
        event.capability = Some(capability);
        event.operation = Some(operation);
        self.record(event)
    }
}
