use agent_kernel_core::{
    AgentId, DriverEndpointDescriptor, DriverEndpointKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceId, ResourceKind,
};

type EndpointKernel<const EVENTS: usize> =
    KernelCore<3, 5, 8, EVENTS, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0>;

fn prepare<const EVENTS: usize>(
    kind: ResourceKind,
    operations: OperationSet,
) -> (
    EndpointKernel<EVENTS>,
    AgentId,
    ResourceId,
    agent_kernel_core::CapabilityId,
) {
    let mut core = EndpointKernel::new();
    let installer = AgentId::new(1);
    core.register_agent(installer).unwrap();
    let resource = core.register_resource(kind, None).unwrap();
    let capability = core
        .grant_capability(installer, resource, operations)
        .unwrap();
    (core, installer, resource, capability)
}

#[test]
fn endpoint_accepts_workspace_control_plane_and_requires_delegate_authority() {
    let (mut workspace_core, installer, workspace, capability) = prepare::<8>(
        ResourceKind::Workspace,
        OperationSet::only(Operation::Delegate),
    );
    workspace_core
        .register_driver_endpoint(
            installer,
            capability,
            workspace,
            DriverEndpointDescriptor::virtual_channel(1),
        )
        .expect("Workspace control plane accepts an explicit endpoint");

    let (mut memory_core, installer, memory, capability) = prepare::<8>(
        ResourceKind::Memory,
        OperationSet::only(Operation::Delegate),
    );
    assert_eq!(
        memory_core.register_driver_endpoint(
            installer,
            capability,
            memory,
            DriverEndpointDescriptor::virtual_channel(1),
        ),
        Err(KernelError::ResourceKindMismatch)
    );

    let (mut device_core, installer, device, capability) =
        prepare::<8>(ResourceKind::Device, OperationSet::only(Operation::Act));
    assert_eq!(
        device_core.register_driver_endpoint(
            installer,
            capability,
            device,
            DriverEndpointDescriptor::virtual_channel(1),
        ),
        Err(KernelError::OperationDenied)
    );
    assert!(device_core.driver_endpoints().is_empty());
}

#[test]
fn endpoint_rejects_duplicate_resource_mapping() {
    let (mut core, installer, device, capability) = prepare::<8>(
        ResourceKind::Device,
        OperationSet::only(Operation::Delegate),
    );
    core.register_driver_endpoint(
        installer,
        capability,
        device,
        DriverEndpointDescriptor::virtual_channel(1),
    )
    .unwrap();
    let events_before = core.events().len();

    let result = core.register_driver_endpoint(
        installer,
        capability,
        device,
        DriverEndpointDescriptor::virtual_channel(2),
    );

    assert_eq!(result, Err(KernelError::DriverEndpointAlreadyExists));
    assert_eq!(core.driver_endpoints().len(), 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn endpoint_rejects_zero_overflowing_and_out_of_range_spans() {
    let invalid = [
        DriverEndpointDescriptor {
            kind: DriverEndpointKind::Mmio,
            base: 1,
            span: 0,
        },
        DriverEndpointDescriptor {
            kind: DriverEndpointKind::Mmio,
            base: u64::MAX,
            span: 2,
        },
        DriverEndpointDescriptor {
            kind: DriverEndpointKind::Port,
            base: u16::MAX as u64,
            span: 2,
        },
    ];

    for descriptor in invalid {
        let (mut core, installer, device, capability) = prepare::<8>(
            ResourceKind::Device,
            OperationSet::only(Operation::Delegate),
        );
        assert_eq!(
            core.register_driver_endpoint(installer, capability, device, descriptor),
            Err(KernelError::DriverEndpointDescriptorInvalid)
        );
        assert!(core.driver_endpoints().is_empty());
    }
}

#[test]
fn endpoint_rejects_same_kind_overlap_but_separates_address_spaces() {
    let mut core = EndpointKernel::<12>::new();
    let installer = AgentId::new(1);
    core.register_agent(installer).unwrap();
    let first = core.register_resource(ResourceKind::Device, None).unwrap();
    let second = core.register_resource(ResourceKind::Device, None).unwrap();
    let third = core.register_resource(ResourceKind::Device, None).unwrap();
    let first_cap = core
        .grant_capability(installer, first, OperationSet::only(Operation::Delegate))
        .unwrap();
    let second_cap = core
        .grant_capability(installer, second, OperationSet::only(Operation::Delegate))
        .unwrap();
    let third_cap = core
        .grant_capability(installer, third, OperationSet::only(Operation::Delegate))
        .unwrap();
    core.register_driver_endpoint(
        installer,
        first_cap,
        first,
        DriverEndpointDescriptor::mmio(0x1000, 0x100),
    )
    .unwrap();

    assert_eq!(
        core.register_driver_endpoint(
            installer,
            second_cap,
            second,
            DriverEndpointDescriptor::mmio(0x1080, 0x20),
        ),
        Err(KernelError::DriverEndpointOverlap)
    );
    core.register_driver_endpoint(
        installer,
        third_cap,
        third,
        DriverEndpointDescriptor::port(0x1000, 0x20),
    )
    .expect("numeric overlap in a separate address space is valid");
    assert_eq!(core.driver_endpoints().len(), 2);
}

#[test]
fn revoked_authority_and_full_log_leave_endpoint_store_unchanged() {
    let (mut revoked, installer, device, capability) = prepare::<8>(
        ResourceKind::Device,
        OperationSet::only(Operation::Delegate),
    );
    revoked.revoke_capability(capability).unwrap();
    assert_eq!(
        revoked.register_driver_endpoint(
            installer,
            capability,
            device,
            DriverEndpointDescriptor::virtual_channel(1),
        ),
        Err(KernelError::CapabilityRevoked)
    );
    assert!(revoked.driver_endpoints().is_empty());

    let (mut full, installer, device, capability) = prepare::<2>(
        ResourceKind::Device,
        OperationSet::only(Operation::Delegate),
    );
    assert_eq!(
        full.register_driver_endpoint(
            installer,
            capability,
            device,
            DriverEndpointDescriptor::virtual_channel(1),
        ),
        Err(KernelError::EventLogFull)
    );
    assert!(full.driver_endpoints().is_empty());
    assert_eq!(full.events().len(), 2);
}

#[test]
fn retired_resource_hides_but_preserves_endpoint_record_for_audit() {
    let (mut core, installer, device, capability) = prepare::<8>(
        ResourceKind::Device,
        OperationSet::empty()
            .with(Operation::Delegate)
            .with(Operation::Rollback),
    );
    core.register_driver_endpoint(
        installer,
        capability,
        device,
        DriverEndpointDescriptor::virtual_channel(1),
    )
    .unwrap();
    core.retire_resource(installer, capability, device).unwrap();

    assert_eq!(
        core.driver_endpoint(device),
        Err(KernelError::ResourceRetired)
    );
    assert_eq!(core.driver_endpoints().len(), 1);
    assert_eq!(core.driver_endpoints()[0].resource, device);
}
