//! Network endpoint and frame-authority syscall facade.
//!
//! This no_std boundary exposes Core network policy while architecture owners
//! retain packet buffers, device queues, and transport registers.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, KernelError, NetworkDatagramDescriptor, NetworkEndpointConfig,
    NetworkEndpointRecord, NetworkFrameDescriptor, NetworkTransferId, NetworkTransferRecord,
    OperationSet, ResourceCreateOutcome, ResourceId,
};

use crate::AgentKernel;

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
    AgentKernel<
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
    pub fn sys_create_network_endpoint(
        &mut self,
        agent: AgentId,
        device_capability: CapabilityId,
        device: ResourceId,
        config: NetworkEndpointConfig,
        operations: OperationSet,
    ) -> Result<ResourceCreateOutcome, KernelError> {
        self.core
            .create_network_endpoint(agent, device_capability, device, config, operations)
    }

    pub fn sys_activate_network_endpoint(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core
            .activate_network_endpoint(agent, capability, endpoint)
    }

    pub fn sys_begin_network_endpoint_revoke(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core
            .begin_network_endpoint_revoke(agent, capability, endpoint)
    }

    pub fn sys_complete_network_endpoint_revoke(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core
            .complete_network_endpoint_revoke(agent, capability, endpoint)
    }

    pub fn sys_prepare_network_transmit(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        frame: NetworkFrameDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        self.core
            .prepare_network_transmit(agent, capability, endpoint, frame)
    }

    pub fn sys_complete_network_transmit(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        transfer: NetworkTransferId,
    ) -> Result<(), KernelError> {
        self.core
            .complete_network_transmit(agent, capability, transfer)
    }

    pub fn sys_prepare_network_datagram_transmit(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        frame: NetworkFrameDescriptor,
        datagram: NetworkDatagramDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        self.core
            .prepare_network_datagram_transmit(agent, capability, endpoint, frame, datagram)
    }

    pub fn sys_fail_network_transmit(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        transfer: NetworkTransferId,
    ) -> Result<(), KernelError> {
        self.core.fail_network_transmit(agent, capability, transfer)
    }

    pub fn sys_record_network_receive(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        frame: NetworkFrameDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        self.core
            .record_network_receive(agent, capability, endpoint, frame)
    }

    pub fn sys_record_network_datagram_receive(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        endpoint: ResourceId,
        frame: NetworkFrameDescriptor,
        datagram: NetworkDatagramDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        self.core
            .record_network_datagram_receive(agent, capability, endpoint, frame, datagram)
    }

    pub fn network_endpoints(&self) -> &[NetworkEndpointRecord] {
        self.core.network_endpoints()
    }

    pub fn network_endpoint(
        &self,
        endpoint: ResourceId,
    ) -> Result<NetworkEndpointRecord, KernelError> {
        self.core.network_endpoint(endpoint)
    }

    pub fn network_transfers(&self) -> &[NetworkTransferRecord] {
        self.core.network_transfers()
    }

    pub fn network_transfer(
        &self,
        transfer: NetworkTransferId,
    ) -> Result<NetworkTransferRecord, KernelError> {
        self.core.network_transfer(transfer)
    }
}
