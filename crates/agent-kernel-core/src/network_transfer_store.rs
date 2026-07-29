//! Fixed-capacity network frame transfer ledger.
//!
//! The ledger binds packet evidence to an active endpoint and explicit
//! Capability authority. It does not retain packet bytes or perform I/O.

use crate::{
    AgentId, CapabilityId, EventKind, KernelCore, KernelError, NetworkEndpointStatus,
    NetworkFrameDescriptor, NetworkTransferDirection, NetworkTransferId, NetworkTransferRecord,
    NetworkTransferStatus, Operation, ResourceId,
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
    pub fn prepare_network_transmit(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        frame: NetworkFrameDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        self.ensure_network_transfer_allowed(agent, capability, endpoint, frame, Operation::Act)?;
        if self.network_transfers().iter().any(|transfer| {
            transfer.endpoint() == endpoint
                && transfer.direction() == NetworkTransferDirection::Transmit
                && transfer.status() == NetworkTransferStatus::Prepared
        }) {
            return Err(KernelError::NetworkTransferPending);
        }
        self.insert_network_transfer(
            agent,
            capability,
            endpoint,
            frame,
            NetworkTransferDirection::Transmit,
            NetworkTransferStatus::Prepared,
            EventKind::NetworkTransmitPrepared,
            Operation::Act,
        )
    }

    pub fn complete_network_transmit(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        transfer: NetworkTransferId,
    ) -> Result<(), KernelError> {
        self.finish_network_transmit(
            agent,
            capability,
            transfer,
            NetworkTransferStatus::Completed,
            EventKind::NetworkTransmitCompleted,
        )
    }

    pub fn fail_network_transmit(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        transfer: NetworkTransferId,
    ) -> Result<(), KernelError> {
        self.finish_network_transmit(
            agent,
            capability,
            transfer,
            NetworkTransferStatus::Failed,
            EventKind::NetworkTransmitFailed,
        )
    }

    pub fn record_network_receive(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        frame: NetworkFrameDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        self.ensure_network_transfer_allowed(
            agent,
            capability,
            endpoint,
            frame,
            Operation::Observe,
        )?;
        self.insert_network_transfer(
            agent,
            capability,
            endpoint,
            frame,
            NetworkTransferDirection::Receive,
            NetworkTransferStatus::Completed,
            EventKind::NetworkReceiveRecorded,
            Operation::Observe,
        )
    }

    pub fn network_transfers(&self) -> &[NetworkTransferRecord] {
        &self.network_transfers[..self.network_transfer_len]
    }

    pub fn network_transfer(
        &self,
        id: NetworkTransferId,
    ) -> Result<NetworkTransferRecord, KernelError> {
        self.network_transfers()
            .iter()
            .find(|transfer| transfer.id() == id)
            .copied()
            .ok_or(KernelError::NetworkTransferNotFound)
    }

    fn ensure_network_transfer_allowed(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        frame: NetworkFrameDescriptor,
        operation: Operation,
    ) -> Result<(), KernelError> {
        self.ensure_agent_active(agent)?;
        let endpoint_record = self.network_endpoint(endpoint)?;
        self.ensure_authorized(agent, capability, endpoint, operation)?;
        if endpoint_record.status() != NetworkEndpointStatus::Active {
            return Err(KernelError::NetworkEndpointStatusMismatch);
        }
        if !endpoint_record.config().accepts(frame) {
            return Err(KernelError::NetworkFrameInvalid);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_network_transfer(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        frame: NetworkFrameDescriptor,
        direction: NetworkTransferDirection,
        status: NetworkTransferStatus,
        event_kind: EventKind,
        operation: Operation,
    ) -> Result<NetworkTransferId, KernelError> {
        if self.network_transfer_len >= CAPS {
            return Err(KernelError::NetworkTransferStoreFull);
        }
        let next = self
            .next_network_transfer
            .checked_add(1)
            .ok_or(KernelError::NetworkTransferStoreFull)?;
        self.ensure_event_slots(1)?;

        let id = NetworkTransferId::new(self.next_network_transfer);
        self.network_transfers[self.network_transfer_len] =
            NetworkTransferRecord::new(id, endpoint, direction, frame, status);
        self.network_transfer_len += 1;
        self.next_network_transfer = next;
        self.record_network_event(event_kind, agent, capability, endpoint, operation)?;
        Ok(id)
    }

    fn finish_network_transmit(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        transfer: NetworkTransferId,
        status: NetworkTransferStatus,
        event_kind: EventKind,
    ) -> Result<(), KernelError> {
        self.ensure_agent_active(agent)?;
        let record = self.network_transfer(transfer)?;
        self.ensure_authorized(agent, capability, record.endpoint(), Operation::Act)?;
        if record.direction() != NetworkTransferDirection::Transmit {
            return Err(KernelError::NetworkTransferDirectionMismatch);
        }
        if record.status() != NetworkTransferStatus::Prepared {
            return Err(KernelError::NetworkTransferStatusMismatch);
        }
        self.ensure_event_slots(1)?;

        let slot = self.network_transfers[..self.network_transfer_len]
            .iter_mut()
            .find(|candidate| candidate.id() == transfer)
            .ok_or(KernelError::NetworkTransferNotFound)?;
        slot.set_status(status);
        self.record_network_event(
            event_kind,
            agent,
            capability,
            record.endpoint(),
            Operation::Act,
        )?;
        Ok(())
    }
}
