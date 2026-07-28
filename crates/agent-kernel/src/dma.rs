//! DMA authority syscall facade.
//!
//! This no_std facade exposes the Core DMA domain, attachment, mapping, and
//! revocation protocol. Architecture code must pair these semantic transitions
//! with its hardware IOMMU owner.

use agent_kernel_core::{
    AgentId, CapabilityId, DmaAccess, DmaAttachmentRecord, DmaDomainRecord, DmaMappingId,
    DmaMappingRecord, DmaRequesterId, Event, KernelError, OperationSet, ResourceCreateOutcome,
    ResourceId,
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
    pub fn sys_create_dma_domain(
        &mut self,
        agent: AgentId,
        iommu_capability: CapabilityId,
        iommu: ResourceId,
        operations: OperationSet,
    ) -> Result<ResourceCreateOutcome, KernelError> {
        self.core
            .create_dma_domain(agent, iommu_capability, iommu, operations)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn sys_attach_dma_device(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        domain: ResourceId,
        device_capability: CapabilityId,
        device: ResourceId,
        requester: DmaRequesterId,
    ) -> Result<Event, KernelError> {
        self.core.attach_dma_device(
            agent,
            domain_capability,
            domain,
            device_capability,
            device,
            requester,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn sys_reserve_dma_mapping(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        domain: ResourceId,
        memory_capability: CapabilityId,
        memory: ResourceId,
        iova: u64,
        page_count: u32,
        access: DmaAccess,
    ) -> Result<DmaMappingId, KernelError> {
        self.core.reserve_dma_mapping(
            agent,
            domain_capability,
            domain,
            memory_capability,
            memory,
            iova,
            page_count,
            access,
        )
    }

    pub fn sys_begin_dma_device_detach(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        domain: ResourceId,
        device_capability: CapabilityId,
        device: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core.begin_dma_device_detach(
            agent,
            domain_capability,
            domain,
            device_capability,
            device,
        )
    }

    pub fn sys_complete_dma_device_detach(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        domain: ResourceId,
        device_capability: CapabilityId,
        device: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core.complete_dma_device_detach(
            agent,
            domain_capability,
            domain,
            device_capability,
            device,
        )
    }

    pub fn sys_activate_dma_mapping(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        mapping: DmaMappingId,
    ) -> Result<Event, KernelError> {
        self.core
            .activate_dma_mapping(agent, domain_capability, mapping)
    }

    pub fn sys_cancel_dma_mapping(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        mapping: DmaMappingId,
    ) -> Result<Event, KernelError> {
        self.core
            .cancel_dma_mapping(agent, domain_capability, mapping)
    }

    pub fn sys_begin_dma_unmap(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        mapping: DmaMappingId,
    ) -> Result<Event, KernelError> {
        self.core.begin_dma_unmap(agent, domain_capability, mapping)
    }

    pub fn sys_complete_dma_unmap(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        mapping: DmaMappingId,
    ) -> Result<Event, KernelError> {
        self.core
            .complete_dma_unmap(agent, domain_capability, mapping)
    }

    pub fn dma_domains(&self) -> &[DmaDomainRecord] {
        self.core.dma_domains()
    }

    pub fn dma_attachments(&self) -> &[DmaAttachmentRecord] {
        self.core.dma_attachments()
    }

    pub fn dma_mappings(&self) -> &[DmaMappingRecord] {
        self.core.dma_mappings()
    }

    pub fn dma_mapping(&self, mapping: DmaMappingId) -> Result<DmaMappingRecord, KernelError> {
        self.core.dma_mapping(mapping)
    }
}
