use agent_kernel_core::{
    AgentId, CapabilityId, DmaAccess, DmaRequesterId, KernelCore, KernelError, Operation,
    OperationSet, ResourceId, ResourceKind, ResourceStatus,
};

type Core = KernelCore<1, 8, 12, 32, 0, 0, 0, 0, 0, 0>;

const OWNER: AgentId = AgentId::new(1);
const IOVA: u64 = 0x0100_0000;

struct Fixture {
    core: Core,
    iommu: ResourceId,
    iommu_capability: CapabilityId,
    device: ResourceId,
    device_capability: CapabilityId,
    memory: ResourceId,
    memory_capability: CapabilityId,
}

fn operations() -> OperationSet {
    OperationSet::empty()
        .with(Operation::Act)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

fn fixture() -> Fixture {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
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

fn attached_domain(fixture: &mut Fixture) -> (ResourceId, CapabilityId) {
    let domain = fixture
        .core
        .create_dma_domain(OWNER, fixture.iommu_capability, fixture.iommu, operations())
        .unwrap();
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
    (domain.resource, domain.capability)
}

#[test]
fn mapped_resources_remain_active_until_revocation_completes() {
    let mut fixture = fixture();
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
    let resources = [
        (fixture.iommu, fixture.iommu_capability),
        (fixture.device, fixture.device_capability),
        (fixture.memory, fixture.memory_capability),
        (domain, domain_capability),
    ];
    let events_before = fixture.core.events().len();

    for (resource, capability) in resources {
        assert_eq!(
            fixture.core.retire_resource(OWNER, capability, resource),
            Err(KernelError::DmaResourceBusy)
        );
    }
    assert_eq!(fixture.core.events().len(), events_before);
    assert!(fixture
        .core
        .resources()
        .iter()
        .all(|resource| resource.status == ResourceStatus::Active));

    fixture
        .core
        .activate_dma_mapping(OWNER, domain_capability, mapping)
        .unwrap();
    fixture
        .core
        .begin_dma_unmap(OWNER, domain_capability, mapping)
        .unwrap();
    for (resource, capability) in resources {
        assert_eq!(
            fixture
                .core
                .can_retire_resource(OWNER, capability, resource),
            Err(KernelError::DmaResourceBusy)
        );
    }

    fixture
        .core
        .complete_dma_unmap(OWNER, domain_capability, mapping)
        .unwrap();
    fixture
        .core
        .can_retire_resource(OWNER, fixture.memory_capability, fixture.memory)
        .expect("released mapping no longer keeps memory busy");
    for (resource, capability) in resources
        .into_iter()
        .filter(|(resource, _)| *resource != fixture.memory)
    {
        assert_eq!(
            fixture
                .core
                .can_retire_resource(OWNER, capability, resource),
            Err(KernelError::DmaResourceBusy)
        );
    }
    fixture
        .core
        .begin_dma_device_detach(
            OWNER,
            domain_capability,
            domain,
            fixture.device_capability,
            fixture.device,
        )
        .unwrap();
    fixture
        .core
        .complete_dma_device_detach(
            OWNER,
            domain_capability,
            domain,
            fixture.device_capability,
            fixture.device,
        )
        .unwrap();
    for (resource, capability) in resources {
        fixture
            .core
            .can_retire_resource(OWNER, capability, resource)
            .expect("released mapping and detached requester are quiescent");
    }
}

#[test]
fn retired_dma_hardware_cannot_accept_new_attachments_or_mappings() {
    let mut iommu_fixture = fixture();
    let domain = iommu_fixture
        .core
        .create_dma_domain(
            OWNER,
            iommu_fixture.iommu_capability,
            iommu_fixture.iommu,
            operations(),
        )
        .unwrap();
    iommu_fixture
        .core
        .retire_resource(OWNER, iommu_fixture.iommu_capability, iommu_fixture.iommu)
        .unwrap();
    assert_eq!(
        iommu_fixture.core.attach_dma_device(
            OWNER,
            domain.capability,
            domain.resource,
            iommu_fixture.device_capability,
            iommu_fixture.device,
            DmaRequesterId::new(0x28),
        ),
        Err(KernelError::ResourceRetired)
    );
    assert!(iommu_fixture.core.dma_attachments().is_empty());

    let mut device_fixture = fixture();
    let (domain, domain_capability) = attached_domain(&mut device_fixture);
    assert_eq!(
        device_fixture.core.retire_resource(
            OWNER,
            device_fixture.device_capability,
            device_fixture.device,
        ),
        Err(KernelError::DmaResourceBusy)
    );
    device_fixture
        .core
        .begin_dma_device_detach(
            OWNER,
            domain_capability,
            domain,
            device_fixture.device_capability,
            device_fixture.device,
        )
        .unwrap();
    device_fixture
        .core
        .complete_dma_device_detach(
            OWNER,
            domain_capability,
            domain,
            device_fixture.device_capability,
            device_fixture.device,
        )
        .unwrap();
    device_fixture
        .core
        .retire_resource(
            OWNER,
            device_fixture.device_capability,
            device_fixture.device,
        )
        .unwrap();
    let events_before = device_fixture.core.events().len();
    assert_eq!(
        device_fixture.core.attach_dma_device(
            OWNER,
            domain_capability,
            domain,
            device_fixture.device_capability,
            device_fixture.device,
            DmaRequesterId::new(0x28),
        ),
        Err(KernelError::ResourceRetired)
    );
    assert_eq!(
        device_fixture.core.reserve_dma_mapping(
            OWNER,
            domain_capability,
            domain,
            device_fixture.memory_capability,
            device_fixture.memory,
            IOVA,
            1,
            DmaAccess::ReadWrite,
        ),
        Err(KernelError::DmaDeviceNotAttached)
    );
    assert!(device_fixture.core.dma_mappings().is_empty());
    assert_eq!(device_fixture.core.events().len(), events_before);
}
