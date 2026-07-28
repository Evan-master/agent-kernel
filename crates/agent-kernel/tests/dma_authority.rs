use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, DmaAccess, DmaAttachmentStatus, DmaMappingStatus, DmaRequesterId, Operation,
    OperationSet, ResourceKind,
};

type Kernel = AgentKernel<1, 8, 12, 32, 0, 0, 0, 0, 0, 0>;

#[test]
fn dma_authority_is_exposed_only_through_the_kernel_facade() {
    let mut kernel = Kernel::new();
    let agent = AgentId::new(1);
    kernel.sys_register_agent(agent).unwrap();
    let operations = OperationSet::only(Operation::Act)
        .with(Operation::Observe)
        .with(Operation::Rollback);
    let iommu = kernel
        .sys_register_resource(ResourceKind::Iommu, None)
        .unwrap();
    let device = kernel
        .sys_register_resource(ResourceKind::Device, None)
        .unwrap();
    let memory = kernel
        .sys_register_resource(ResourceKind::Memory, None)
        .unwrap();
    let iommu_capability = kernel.sys_grant(agent, iommu, operations).unwrap();
    let device_capability = kernel.sys_grant(agent, device, operations).unwrap();
    let memory_capability = kernel.sys_grant(agent, memory, operations).unwrap();

    let domain = kernel
        .sys_create_dma_domain(agent, iommu_capability, iommu, operations)
        .unwrap();
    kernel
        .sys_attach_dma_device(
            agent,
            domain.capability,
            domain.resource,
            device_capability,
            device,
            DmaRequesterId::new(0x28),
        )
        .unwrap();
    let mapping = kernel
        .sys_reserve_dma_mapping(
            agent,
            domain.capability,
            domain.resource,
            memory_capability,
            memory,
            0x0100_0000,
            1,
            DmaAccess::ReadWrite,
        )
        .unwrap();
    kernel
        .sys_activate_dma_mapping(agent, domain.capability, mapping)
        .unwrap();

    assert_eq!(
        kernel.dma_mapping(mapping).unwrap().status,
        DmaMappingStatus::Active
    );
    kernel
        .sys_begin_dma_device_detach(
            agent,
            domain.capability,
            domain.resource,
            device_capability,
            device,
        )
        .unwrap();
    kernel
        .sys_complete_dma_device_detach(
            agent,
            domain.capability,
            domain.resource,
            device_capability,
            device,
        )
        .unwrap();
    assert_eq!(
        kernel
            .dma_attachments()
            .last()
            .expect("one DMA attachment")
            .status(),
        DmaAttachmentStatus::Detached
    );
}
