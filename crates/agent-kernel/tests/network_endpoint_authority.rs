use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, NetworkEndpointConfig, NetworkEndpointStatus, NetworkFrameDescriptor,
    NetworkMacAddress, NetworkTransferStatus, Operation, OperationSet, ResourceKind,
};

type Kernel = AgentKernel<1, 8, 12, 32, 0, 0, 0, 0, 0, 0>;

#[test]
fn network_authority_is_exposed_through_the_kernel_facade() {
    let mut kernel = Kernel::new();
    let agent = AgentId::new(1);
    kernel.sys_register_agent(agent).unwrap();
    let operations = OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);
    let device = kernel
        .sys_register_resource(ResourceKind::Device, None)
        .unwrap();
    let device_capability = kernel.sys_grant(agent, device, operations).unwrap();
    let config = NetworkEndpointConfig::new(
        NetworkMacAddress::new([0x52, 0x54, 0, 0x12, 0x34, 0x56]).unwrap(),
        1500,
    )
    .unwrap();

    let endpoint = kernel
        .sys_create_network_endpoint(agent, device_capability, device, config, operations)
        .unwrap();
    kernel
        .sys_activate_network_endpoint(agent, endpoint.capability, endpoint.resource)
        .unwrap();
    let transfer = kernel
        .sys_prepare_network_transmit(
            agent,
            endpoint.capability,
            endpoint.resource,
            NetworkFrameDescriptor::new(60, 0x0806, [0x11; 32]).unwrap(),
        )
        .unwrap();
    kernel
        .sys_complete_network_transmit(agent, endpoint.capability, transfer)
        .unwrap();

    assert_eq!(
        kernel.network_endpoint(endpoint.resource).unwrap().status(),
        NetworkEndpointStatus::Active
    );
    assert_eq!(
        kernel.network_transfer(transfer).unwrap().status(),
        NetworkTransferStatus::Completed
    );
}
