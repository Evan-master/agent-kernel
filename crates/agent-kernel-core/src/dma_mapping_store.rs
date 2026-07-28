//! Fixed-capacity DMA mapping lifecycle.
//!
//! This core module reserves architecture-neutral IOVA ranges and applies the
//! activation and two-phase revocation protocol. It owns no physical addresses
//! and performs no hardware mutation.

use crate::{
    AgentId, CapabilityId, DmaAccess, DmaMappingId, DmaMappingRecord, DmaMappingStatus, Event,
    EventKind, KernelCore, KernelError, Operation, ResourceId, ResourceKind, DMA_PAGE_BYTES,
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
    #[allow(clippy::too_many_arguments)]
    pub fn reserve_dma_mapping(
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
        self.ensure_agent_active(agent)?;
        let domain_record = self.find_dma_domain(domain)?;
        self.ensure_authorized(agent, domain_capability, domain, Operation::Act)?;
        self.find_resource(domain_record.iommu)?;
        let mut has_attached_device = false;
        for attachment in self
            .dma_attachments()
            .iter()
            .filter(|attachment| attachment.domain == domain)
        {
            has_attached_device = true;
            self.find_resource(attachment.device)?;
        }
        if !has_attached_device {
            return Err(KernelError::DmaDeviceNotAttached);
        }
        if self.find_resource(memory)?.kind != ResourceKind::Memory {
            return Err(KernelError::ResourceKindMismatch);
        }
        self.ensure_authorized(agent, memory_capability, memory, Operation::Act)?;
        let end = dma_range_end(iova, page_count)?;
        if self.dma_mapping_len >= CAPS {
            return Err(KernelError::DmaMappingStoreFull);
        }
        if self.dma_mappings().iter().any(|mapping| {
            mapping.domain == domain
                && mapping.occupies_iova()
                && ranges_overlap(iova, end, mapping.iova, mapping.end_iova())
        }) {
            return Err(KernelError::DmaMappingOverlap);
        }
        self.ensure_event_slots(1)?;

        let id = DmaMappingId::new(self.next_dma_mapping);
        self.next_dma_mapping += 1;
        self.dma_mappings[self.dma_mapping_len] = DmaMappingRecord {
            id,
            domain,
            memory,
            iova,
            page_count,
            access,
            status: DmaMappingStatus::Reserved,
        };
        self.dma_mapping_len += 1;
        self.record_dma_event(
            EventKind::DmaMappingReserved,
            agent,
            memory,
            domain_capability,
            Some(memory_capability),
        )?;
        Ok(id)
    }

    pub fn activate_dma_mapping(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        mapping: DmaMappingId,
    ) -> Result<Event, KernelError> {
        self.transition_dma_mapping(
            agent,
            domain_capability,
            mapping,
            DmaMappingStatus::Reserved,
            DmaMappingStatus::Active,
            EventKind::DmaMappingActivated,
        )
    }

    pub fn cancel_dma_mapping(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        mapping: DmaMappingId,
    ) -> Result<Event, KernelError> {
        self.transition_dma_mapping(
            agent,
            domain_capability,
            mapping,
            DmaMappingStatus::Reserved,
            DmaMappingStatus::Cancelled,
            EventKind::DmaMappingCancelled,
        )
    }

    pub fn begin_dma_unmap(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        mapping: DmaMappingId,
    ) -> Result<Event, KernelError> {
        self.transition_dma_mapping(
            agent,
            domain_capability,
            mapping,
            DmaMappingStatus::Active,
            DmaMappingStatus::Revoking,
            EventKind::DmaMappingRevoking,
        )
    }

    pub fn complete_dma_unmap(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        mapping: DmaMappingId,
    ) -> Result<Event, KernelError> {
        self.transition_dma_mapping(
            agent,
            domain_capability,
            mapping,
            DmaMappingStatus::Revoking,
            DmaMappingStatus::Released,
            EventKind::DmaMappingReleased,
        )
    }

    pub fn dma_mappings(&self) -> &[DmaMappingRecord] {
        &self.dma_mappings[..self.dma_mapping_len]
    }

    pub fn dma_mapping(&self, id: DmaMappingId) -> Result<DmaMappingRecord, KernelError> {
        self.dma_mappings()
            .iter()
            .find(|mapping| mapping.id == id)
            .copied()
            .ok_or(KernelError::DmaMappingNotFound)
    }

    pub(crate) fn ensure_dma_resource_quiescent(
        &self,
        resource: ResourceId,
    ) -> Result<(), KernelError> {
        let busy =
            self.dma_mappings()
                .iter()
                .filter(|mapping| mapping.occupies_iova())
                .any(|mapping| {
                    mapping.domain == resource
                        || mapping.memory == resource
                        || self.dma_domains().iter().any(|domain| {
                            domain.resource == mapping.domain && domain.iommu == resource
                        })
                        || self.dma_attachments().iter().any(|attachment| {
                            attachment.domain == mapping.domain && attachment.device == resource
                        })
                });
        if busy {
            Err(KernelError::DmaResourceBusy)
        } else {
            Ok(())
        }
    }

    fn transition_dma_mapping(
        &mut self,
        agent: AgentId,
        domain_capability: CapabilityId,
        id: DmaMappingId,
        expected: DmaMappingStatus,
        next: DmaMappingStatus,
        event_kind: EventKind,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let mapping = self.dma_mapping(id)?;
        self.ensure_authorized(agent, domain_capability, mapping.domain, Operation::Act)?;
        if mapping.status != expected {
            return Err(KernelError::DmaMappingStatusMismatch);
        }
        self.ensure_event_slots(1)?;
        let slot = self.dma_mappings[..self.dma_mapping_len]
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or(KernelError::DmaMappingNotFound)?;
        slot.status = next;
        self.record_dma_event(event_kind, agent, mapping.memory, domain_capability, None)
    }
}

fn dma_range_end(iova: u64, page_count: u32) -> Result<u64, KernelError> {
    if !iova.is_multiple_of(DMA_PAGE_BYTES) || page_count == 0 {
        return Err(KernelError::DmaMappingInvalid);
    }
    iova.checked_add(
        u64::from(page_count)
            .checked_mul(DMA_PAGE_BYTES)
            .ok_or(KernelError::DmaMappingInvalid)?,
    )
    .ok_or(KernelError::DmaMappingInvalid)
}

fn ranges_overlap(
    first_start: u64,
    first_end: u64,
    second_start: u64,
    second_end: Option<u64>,
) -> bool {
    match second_end {
        Some(second_end) => first_start < second_end && second_start < first_end,
        None => true,
    }
}
