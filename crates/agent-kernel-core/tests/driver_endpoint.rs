use agent_kernel_core::{
    AgentId, DriverEndpointDescriptor, DriverEndpointKind, EventKind, KernelCore, Operation,
    OperationSet, ResourceKind,
};

type EndpointKernel = KernelCore<3, 4, 8, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0>;

#[test]
fn authorized_installer_registers_queryable_virtual_endpoint() {
    let mut core = EndpointKernel::new();
    let installer = AgentId::new(1);
    core.register_agent(installer).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core
        .grant_capability(installer, device, OperationSet::only(Operation::Delegate))
        .unwrap();
    let descriptor = DriverEndpointDescriptor::virtual_channel(7);

    let event = core
        .register_driver_endpoint(installer, capability, device, descriptor)
        .expect("delegated installer should register endpoint");

    assert_eq!(event.kind, EventKind::DriverEndpointRegistered);
    assert_eq!(event.agent, installer);
    assert_eq!(event.resource, Some(device));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.operation, Some(Operation::Delegate));
    assert_eq!(core.driver_endpoints().len(), 1);
    assert_eq!(core.driver_endpoints()[0].resource, device);
    assert_eq!(core.driver_endpoints()[0].installer, installer);
    assert_eq!(core.driver_endpoints()[0].descriptor, descriptor);
    assert_eq!(core.driver_endpoint(device).unwrap().descriptor, descriptor);
    assert_eq!(descriptor.kind, DriverEndpointKind::Virtual);
    assert_eq!(descriptor.base, 7);
    assert_eq!(descriptor.span, 1);
}
