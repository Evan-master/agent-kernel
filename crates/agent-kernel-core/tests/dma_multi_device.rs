use agent_kernel_core::{
    AgentId, DmaAccess, DmaAttachmentStatus, DmaRequesterId, EventKind, KernelCore, KernelError,
    Operation, OperationSet, ResourceKind, ResourceStatus,
};

type Core = KernelCore<2, 12, 20, 64, 0, 0, 0, 0, 0, 0>;

const OWNER: AgentId = AgentId::new(1);
const OTHER: AgentId = AgentId::new(2);
const IOVA: u64 = 0x0100_0000;

fn operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

#[test]
fn one_domain_accepts_two_devices_and_detaches_one_in_two_phases() {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let iommu = core.register_resource(ResourceKind::Iommu, None).unwrap();
    let first_device = core.register_resource(ResourceKind::Device, None).unwrap();
    let second_device = core.register_resource(ResourceKind::Device, None).unwrap();
    let memory = core.register_resource(ResourceKind::Memory, None).unwrap();
    let iommu_capability = core.grant_capability(OWNER, iommu, operations()).unwrap();
    let first_capability = core
        .grant_capability(OWNER, first_device, operations())
        .unwrap();
    let second_capability = core
        .grant_capability(OWNER, second_device, operations())
        .unwrap();
    let memory_capability = core.grant_capability(OWNER, memory, operations()).unwrap();
    let domain = core
        .create_dma_domain(OWNER, iommu_capability, iommu, operations())
        .unwrap();
    let first_dma_event = core.events().len();

    core.attach_dma_device(
        OWNER,
        domain.capability,
        domain.resource,
        first_capability,
        first_device,
        DmaRequesterId::new(0x28),
    )
    .unwrap();
    core.attach_dma_device(
        OWNER,
        domain.capability,
        domain.resource,
        second_capability,
        second_device,
        DmaRequesterId::new(0x30),
    )
    .unwrap();
    core.reserve_dma_mapping(
        OWNER,
        domain.capability,
        domain.resource,
        memory_capability,
        memory,
        IOVA,
        1,
        DmaAccess::ReadWrite,
    )
    .unwrap();

    core.begin_dma_device_detach(
        OWNER,
        domain.capability,
        domain.resource,
        second_capability,
        second_device,
    )
    .unwrap();
    assert_eq!(
        core.dma_attachment(second_device).unwrap().status(),
        DmaAttachmentStatus::Detaching
    );
    core.complete_dma_device_detach(
        OWNER,
        domain.capability,
        domain.resource,
        second_capability,
        second_device,
    )
    .unwrap();
    assert_eq!(
        core.dma_attachment(second_device).unwrap().status(),
        DmaAttachmentStatus::Detached
    );
    assert_eq!(
        core.dma_attachment(first_device).unwrap().status(),
        DmaAttachmentStatus::Attached
    );
    assert_eq!(
        core.events()[first_dma_event..]
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [
            EventKind::DmaDeviceAttached,
            EventKind::DmaDeviceAttached,
            EventKind::DmaMappingReserved,
            EventKind::DmaDeviceDetaching,
            EventKind::DmaDeviceDetached,
        ]
    );
}

#[test]
fn detached_device_and_requester_can_be_attached_again() {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let iommu = core.register_resource(ResourceKind::Iommu, None).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let iommu_capability = core.grant_capability(OWNER, iommu, operations()).unwrap();
    let device_capability = core.grant_capability(OWNER, device, operations()).unwrap();
    let domain = core
        .create_dma_domain(OWNER, iommu_capability, iommu, operations())
        .unwrap();

    core.attach_dma_device(
        OWNER,
        domain.capability,
        domain.resource,
        device_capability,
        device,
        DmaRequesterId::new(0x28),
    )
    .unwrap();
    core.begin_dma_device_detach(
        OWNER,
        domain.capability,
        domain.resource,
        device_capability,
        device,
    )
    .unwrap();
    core.complete_dma_device_detach(
        OWNER,
        domain.capability,
        domain.resource,
        device_capability,
        device,
    )
    .unwrap();
    core.attach_dma_device(
        OWNER,
        domain.capability,
        domain.resource,
        device_capability,
        device,
        DmaRequesterId::new(0x28),
    )
    .unwrap();

    assert_eq!(core.dma_attachments().len(), 2);
    assert_eq!(
        core.dma_attachment(device).unwrap().status(),
        DmaAttachmentStatus::Attached
    );
}

#[test]
fn detach_requires_exact_domain_and_device_authority() {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    core.register_agent(OTHER).unwrap();
    let iommu = core.register_resource(ResourceKind::Iommu, None).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let iommu_capability = core.grant_capability(OWNER, iommu, operations()).unwrap();
    let device_capability = core.grant_capability(OWNER, device, operations()).unwrap();
    let domain = core
        .create_dma_domain(OWNER, iommu_capability, iommu, operations())
        .unwrap();
    core.attach_dma_device(
        OWNER,
        domain.capability,
        domain.resource,
        device_capability,
        device,
        DmaRequesterId::new(0x28),
    )
    .unwrap();
    let event_count = core.events().len();

    assert_eq!(
        core.begin_dma_device_detach(
            OTHER,
            domain.capability,
            domain.resource,
            device_capability,
            device,
        ),
        Err(KernelError::AgentMismatch)
    );
    assert_eq!(
        core.complete_dma_device_detach(
            OWNER,
            domain.capability,
            domain.resource,
            device_capability,
            device,
        ),
        Err(KernelError::DmaAttachmentStatusMismatch)
    );
    assert_eq!(core.events().len(), event_count);
    assert_eq!(
        core.dma_attachment(device).unwrap().status(),
        DmaAttachmentStatus::Attached
    );
}

#[test]
fn live_attachments_block_resource_retirement_without_mappings() {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let iommu = core.register_resource(ResourceKind::Iommu, None).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let iommu_capability = core.grant_capability(OWNER, iommu, operations()).unwrap();
    let device_capability = core.grant_capability(OWNER, device, operations()).unwrap();
    let domain = core
        .create_dma_domain(OWNER, iommu_capability, iommu, operations())
        .unwrap();
    core.attach_dma_device(
        OWNER,
        domain.capability,
        domain.resource,
        device_capability,
        device,
        DmaRequesterId::new(0x28),
    )
    .unwrap();

    assert_eq!(
        core.retire_resource(OWNER, device_capability, device),
        Err(KernelError::DmaResourceBusy)
    );
    assert_eq!(
        core.retire_resource(OWNER, domain.capability, domain.resource),
        Err(KernelError::DmaResourceBusy)
    );
    assert_eq!(
        core.retire_resource(OWNER, iommu_capability, iommu),
        Err(KernelError::DmaResourceBusy)
    );

    core.begin_dma_device_detach(
        OWNER,
        domain.capability,
        domain.resource,
        device_capability,
        device,
    )
    .unwrap();
    assert_eq!(
        core.retire_resource(OWNER, device_capability, device),
        Err(KernelError::DmaResourceBusy)
    );
    core.complete_dma_device_detach(
        OWNER,
        domain.capability,
        domain.resource,
        device_capability,
        device,
    )
    .unwrap();
    core.retire_resource(OWNER, device_capability, device)
        .unwrap();
    assert_eq!(
        core.resources()
            .iter()
            .find(|resource| resource.id == device)
            .unwrap()
            .status,
        ResourceStatus::Retired
    );
}
