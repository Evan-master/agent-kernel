//! Fixed-capacity DMA domain and device attachment state.
//!
//! This core module creates DMA-domain Resources and binds device Resources to
//! opaque requester identities. Every mutation requires explicit Capabilities
//! and appends one DMA-specific Event.

use crate::{
    AgentId, CapabilityId, DmaAttachmentRecord, DmaAttachmentStatus, DmaDomainRecord,
    DmaRequesterId, Event, EventKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceCreateOutcome, ResourceId, ResourceKind,
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
    pub fn create_dma_domain(
        &mut self,
        agent: AgentId,
        iommu_capability: CapabilityId,
        iommu: ResourceId,
        operations: OperationSet,
    ) -> Result<ResourceCreateOutcome, KernelError> {
        self.ensure_agent_active(agent)?;
        if self.find_resource(iommu)?.kind != ResourceKind::Iommu {
            return Err(KernelError::ResourceKindMismatch);
        }
        self.ensure_authorized(agent, iommu_capability, iommu, Operation::Act)?;
        if self.dma_domain_len >= RESOURCES {
            return Err(KernelError::DmaDomainStoreFull);
        }
        self.ensure_event_slots(3)?;

        let outcome = self.create_resource(
            agent,
            ResourceKind::DmaDomain,
            Some((iommu, iommu_capability)),
            operations,
        )?;
        self.dma_domains[self.dma_domain_len] = DmaDomainRecord {
            resource: outcome.resource,
            iommu,
            owner: agent,
        };
        self.dma_domain_len += 1;
        self.record_dma_event(
            EventKind::DmaDomainCreated,
            agent,
            outcome.resource,
            outcome.capability,
            Some(iommu_capability),
        )?;
        Ok(outcome)
    }

    pub fn attach_dma_device(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        domain: ResourceId,
        device_capability: CapabilityId,
        device: ResourceId,
        requester: DmaRequesterId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let domain_record = self.find_dma_domain(domain)?;
        self.ensure_authorized(agent, domain_capability, domain, Operation::Act)?;
        self.find_resource(domain_record.iommu)?;
        if self.find_resource(device)?.kind != ResourceKind::Device {
            return Err(KernelError::ResourceKindMismatch);
        }
        self.ensure_authorized(agent, device_capability, device, Operation::Act)?;
        if self
            .dma_attachments()
            .iter()
            .any(|attachment| attachment.occupies_attachment() && attachment.device == device)
        {
            return Err(KernelError::DmaDeviceAlreadyAttached);
        }
        if self
            .dma_attachments()
            .iter()
            .any(|attachment| attachment.occupies_attachment() && attachment.requester == requester)
        {
            return Err(KernelError::DmaRequesterAlreadyAttached);
        }
        if self.dma_attachment_len >= RESOURCES {
            return Err(KernelError::DmaAttachmentStoreFull);
        }
        self.ensure_event_slots(1)?;

        self.dma_attachments[self.dma_attachment_len] = DmaAttachmentRecord {
            domain,
            device,
            requester,
            status: DmaAttachmentStatus::Attached,
        };
        self.dma_attachment_len += 1;
        self.record_dma_event(
            EventKind::DmaDeviceAttached,
            agent,
            device,
            domain_capability,
            Some(device_capability),
        )
    }

    pub fn begin_dma_device_detach(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        domain: ResourceId,
        device_capability: CapabilityId,
        device: ResourceId,
    ) -> Result<Event, KernelError> {
        self.transition_dma_attachment(
            agent,
            domain_capability,
            domain,
            device_capability,
            device,
            DmaAttachmentStatus::Attached,
            DmaAttachmentStatus::Detaching,
            EventKind::DmaDeviceDetaching,
        )
    }

    pub fn complete_dma_device_detach(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        domain: ResourceId,
        device_capability: CapabilityId,
        device: ResourceId,
    ) -> Result<Event, KernelError> {
        self.transition_dma_attachment(
            agent,
            domain_capability,
            domain,
            device_capability,
            device,
            DmaAttachmentStatus::Detaching,
            DmaAttachmentStatus::Detached,
            EventKind::DmaDeviceDetached,
        )
    }

    pub fn dma_domains(&self) -> &[DmaDomainRecord] {
        &self.dma_domains[..self.dma_domain_len]
    }

    pub fn dma_attachments(&self) -> &[DmaAttachmentRecord] {
        &self.dma_attachments[..self.dma_attachment_len]
    }

    pub fn dma_domain(&self, resource: ResourceId) -> Result<DmaDomainRecord, KernelError> {
        self.find_dma_domain(resource)
    }

    pub fn dma_attachment(&self, device: ResourceId) -> Result<DmaAttachmentRecord, KernelError> {
        self.dma_attachments()
            .iter()
            .rev()
            .find(|attachment| attachment.device == device)
            .copied()
            .ok_or(KernelError::DmaDeviceNotAttached)
    }

    pub(crate) fn find_dma_domain(
        &self,
        resource: ResourceId,
    ) -> Result<DmaDomainRecord, KernelError> {
        self.dma_domains()
            .iter()
            .find(|domain| domain.resource == resource)
            .copied()
            .ok_or(KernelError::DmaDomainNotFound)
    }

    #[allow(clippy::too_many_arguments)]
    fn transition_dma_attachment(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        domain: ResourceId,
        device_capability: CapabilityId,
        device: ResourceId,
        expected: DmaAttachmentStatus,
        next: DmaAttachmentStatus,
        event_kind: EventKind,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let domain_record = self.find_dma_domain(domain)?;
        self.find_resource(domain_record.iommu)?;
        self.ensure_authorized(agent, domain_capability, domain, Operation::Rollback)?;
        if self.find_resource(device)?.kind != ResourceKind::Device {
            return Err(KernelError::ResourceKindMismatch);
        }
        self.ensure_authorized(agent, device_capability, device, Operation::Rollback)?;
        let index = (0..self.dma_attachment_len)
            .rev()
            .find(|index| {
                let attachment = self.dma_attachments[*index];
                attachment.domain == domain && attachment.device == device
            })
            .ok_or(KernelError::DmaDeviceNotAttached)?;
        if self.dma_attachments[index].status != expected {
            return Err(KernelError::DmaAttachmentStatusMismatch);
        }
        self.ensure_event_slots(1)?;
        self.dma_attachments[index].status = next;
        self.record_dma_event_with_operation(
            event_kind,
            agent,
            device,
            domain_capability,
            Some(device_capability),
            Operation::Rollback,
        )
    }
}
