use agent_kernel_core::{
    AgentId, CapabilityId, DmaAccess, DmaMappingStatus, DmaRequesterId, EventKind, KernelCore,
    KernelError, Operation, OperationSet, ResourceId, ResourceKind,
};

type Core<const CAPS: usize = 12, const EVENTS: usize = 32> =
    KernelCore<2, 8, CAPS, EVENTS, 0, 0, 0, 0, 0, 0>;

const OWNER: AgentId = AgentId::new(1);
const OTHER: AgentId = AgentId::new(2);
const IOVA: u64 = 0x0100_0000;

struct Fixture<const CAPS: usize = 12, const EVENTS: usize = 32> {
    core: Core<CAPS, EVENTS>,
    iommu: ResourceId,
    iommu_capability: CapabilityId,
    device: ResourceId,
    device_capability: CapabilityId,
    memory: ResourceId,
    memory_capability: CapabilityId,
}

fn operations() -> OperationSet {
    OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

fn fixture<const CAPS: usize, const EVENTS: usize>() -> Fixture<CAPS, EVENTS> {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    core.register_agent(OTHER).unwrap();
    let iommu = core.register_resource(ResourceKind::Iommu, None).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let memory = core.register_resource(ResourceKind::Memory, None).unwrap();
    let iommu_capability = core.grant_capability(OWNER, iommu, operations()).unwrap();
    let device_capability = core.grant_capability(OWNER, device, operations()).unwrap();
    let memory_capability = core.grant_capability(OWNER, memory, operations()).unwrap();
    Fixture {
        core,
        iommu,
        iommu_capability,
        device,
        device_capability,
        memory,
        memory_capability,
    }
}

fn attached_domain<const CAPS: usize, const EVENTS: usize>(
    fixture: &mut Fixture<CAPS, EVENTS>,
) -> (ResourceId, CapabilityId) {
    let outcome = fixture
        .core
        .create_dma_domain(OWNER, fixture.iommu_capability, fixture.iommu, operations())
        .unwrap();
    fixture
        .core
        .attach_dma_device(
            OWNER,
            outcome.capability,
            outcome.resource,
            fixture.device_capability,
            fixture.device,
            DmaRequesterId::new(0x28),
        )
        .unwrap();
    (outcome.resource, outcome.capability)
}

#[test]
fn dma_mapping_lifecycle_is_capability_checked_and_evented() {
    let mut fixture = fixture::<12, 32>();
    let (domain, domain_capability) = attached_domain(&mut fixture);
    let first_dma_event = fixture.core.events().len() - 2;

    let mapping = fixture
        .core
        .reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            fixture.memory_capability,
            fixture.memory,
            IOVA,
            1,
            DmaAccess::ReadWrite,
        )
        .unwrap();
    fixture
        .core
        .activate_dma_mapping(OWNER, domain_capability, mapping)
        .unwrap();
    fixture
        .core
        .begin_dma_unmap(OWNER, domain_capability, mapping)
        .unwrap();
    fixture
        .core
        .complete_dma_unmap(OWNER, domain_capability, mapping)
        .unwrap();

    assert_eq!(
        fixture.core.dma_mapping(mapping).unwrap().status,
        DmaMappingStatus::Released
    );
    assert_eq!(
        fixture.core.events()[first_dma_event..]
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [
            EventKind::DmaDomainCreated,
            EventKind::DmaDeviceAttached,
            EventKind::DmaMappingReserved,
            EventKind::DmaMappingActivated,
            EventKind::DmaMappingRevoking,
            EventKind::DmaMappingReleased,
        ]
    );
}

#[test]
fn dma_rejects_wrong_authority_overlap_and_invalid_transitions_atomically() {
    let mut fixture = fixture::<12, 32>();
    let (domain, domain_capability) = attached_domain(&mut fixture);
    let mapping = fixture
        .core
        .reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            fixture.memory_capability,
            fixture.memory,
            IOVA,
            2,
            DmaAccess::ReadWrite,
        )
        .unwrap();
    let event_count = fixture.core.events().len();

    assert_eq!(
        fixture.core.reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            fixture.memory_capability,
            fixture.memory,
            IOVA + 4096,
            1,
            DmaAccess::Read,
        ),
        Err(KernelError::DmaMappingOverlap)
    );
    assert_eq!(
        fixture
            .core
            .activate_dma_mapping(OTHER, domain_capability, mapping),
        Err(KernelError::AgentMismatch)
    );
    assert_eq!(
        fixture
            .core
            .complete_dma_unmap(OWNER, domain_capability, mapping),
        Err(KernelError::DmaMappingStatusMismatch)
    );
    assert_eq!(fixture.core.events().len(), event_count);
    assert_eq!(
        fixture.core.dma_mapping(mapping).unwrap().status,
        DmaMappingStatus::Reserved
    );
}

#[test]
fn cancelled_reservation_cannot_be_activated() {
    let mut fixture = fixture::<12, 32>();
    let (domain, domain_capability) = attached_domain(&mut fixture);
    let mapping = fixture
        .core
        .reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            fixture.memory_capability,
            fixture.memory,
            IOVA,
            1,
            DmaAccess::Write,
        )
        .unwrap();

    fixture
        .core
        .cancel_dma_mapping(OWNER, domain_capability, mapping)
        .unwrap();
    assert_eq!(
        fixture
            .core
            .activate_dma_mapping(OWNER, domain_capability, mapping),
        Err(KernelError::DmaMappingStatusMismatch)
    );
    assert_eq!(
        fixture.core.dma_mapping(mapping).unwrap().status,
        DmaMappingStatus::Cancelled
    );
    assert_eq!(
        fixture.core.events().last().unwrap().kind,
        EventKind::DmaMappingCancelled
    );
}

#[test]
fn revoking_mapping_keeps_its_iova_until_release_completes() {
    let mut fixture = fixture::<12, 32>();
    let (domain, domain_capability) = attached_domain(&mut fixture);
    let mapping = fixture
        .core
        .reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            fixture.memory_capability,
            fixture.memory,
            IOVA,
            1,
            DmaAccess::ReadWrite,
        )
        .unwrap();
    fixture
        .core
        .activate_dma_mapping(OWNER, domain_capability, mapping)
        .unwrap();
    fixture
        .core
        .begin_dma_unmap(OWNER, domain_capability, mapping)
        .unwrap();

    assert_eq!(
        fixture.core.reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            fixture.memory_capability,
            fixture.memory,
            IOVA,
            1,
            DmaAccess::ReadWrite,
        ),
        Err(KernelError::DmaMappingOverlap)
    );

    fixture
        .core
        .complete_dma_unmap(OWNER, domain_capability, mapping)
        .unwrap();
    fixture
        .core
        .reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            fixture.memory_capability,
            fixture.memory,
            IOVA,
            1,
            DmaAccess::ReadWrite,
        )
        .expect("released IOVA can be reserved again");
}

#[test]
fn mapping_requires_an_attachment_and_valid_page_range() {
    let mut fixture = fixture::<12, 32>();
    let domain = fixture
        .core
        .create_dma_domain(OWNER, fixture.iommu_capability, fixture.iommu, operations())
        .unwrap();
    let event_count = fixture.core.events().len();

    assert_eq!(
        fixture.core.reserve_dma_mapping(
            OWNER,
            domain.capability,
            domain.resource,
            fixture.memory_capability,
            fixture.memory,
            IOVA,
            1,
            DmaAccess::Read,
        ),
        Err(KernelError::DmaDeviceNotAttached)
    );
    fixture
        .core
        .attach_dma_device(
            OWNER,
            domain.capability,
            domain.resource,
            fixture.device_capability,
            fixture.device,
            DmaRequesterId::new(0x28),
        )
        .unwrap();
    let after_attachment = fixture.core.events().len();
    assert_eq!(after_attachment, event_count + 1);
    assert_eq!(
        fixture.core.reserve_dma_mapping(
            OWNER,
            domain.capability,
            domain.resource,
            fixture.memory_capability,
            fixture.memory,
            IOVA + 1,
            1,
            DmaAccess::Read,
        ),
        Err(KernelError::DmaMappingInvalid)
    );
    assert_eq!(fixture.core.events().len(), after_attachment);
}

#[test]
fn mapping_store_capacity_fails_without_partial_state() {
    let mut fixture = fixture::<4, 32>();
    let (domain, domain_capability) = attached_domain(&mut fixture);
    for index in 0..4 {
        fixture
            .core
            .reserve_dma_mapping(
                OWNER,
                domain_capability,
                domain,
                fixture.memory_capability,
                fixture.memory,
                IOVA + index * 4096,
                1,
                DmaAccess::ReadWrite,
            )
            .unwrap();
    }
    let event_count = fixture.core.events().len();

    assert_eq!(
        fixture.core.reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            fixture.memory_capability,
            fixture.memory,
            IOVA + 4 * 4096,
            1,
            DmaAccess::ReadWrite,
        ),
        Err(KernelError::DmaMappingStoreFull)
    );
    assert_eq!(fixture.core.dma_mappings().len(), 4);
    assert_eq!(fixture.core.events().len(), event_count);
}
