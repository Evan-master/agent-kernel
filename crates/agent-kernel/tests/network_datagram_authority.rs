use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, NetworkDatagramDescriptor, NetworkEndpointConfig, NetworkFrameDescriptor,
    NetworkIpv4Address, NetworkMacAddress, NetworkTransferStatus, NetworkUdpPort, Operation,
    OperationSet, ResourceKind,
};

type Kernel = AgentKernel<1, 8, 12, 32, 0, 0, 0, 0, 0, 0>;

#[test]
fn ipv4_udp_authority_is_exposed_through_the_kernel_facade() {
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
    let endpoint = kernel
        .sys_create_network_endpoint(
            agent,
            device_capability,
            device,
            NetworkEndpointConfig::new(
                NetworkMacAddress::new([0x52, 0x54, 0, 0x12, 0x34, 0x56]).unwrap(),
                1500,
            )
            .unwrap(),
            operations,
        )
        .unwrap();
    kernel
        .sys_activate_network_endpoint(agent, endpoint.capability, endpoint.resource)
        .unwrap();
    let datagram = NetworkDatagramDescriptor::new(
        NetworkIpv4Address::new([10, 0, 2, 15]).unwrap(),
        NetworkIpv4Address::new([10, 0, 2, 2]).unwrap(),
        NetworkUdpPort::new(40131).unwrap(),
        NetworkUdpPort::new(40130).unwrap(),
        13,
        [0x11; 32],
    )
    .unwrap();
    let transfer = kernel
        .sys_prepare_network_datagram_transmit(
            agent,
            endpoint.capability,
            endpoint.resource,
            NetworkFrameDescriptor::new(60, 0x0800, [0x22; 32]).unwrap(),
            datagram,
        )
        .unwrap();
    kernel
        .sys_complete_network_transmit(agent, endpoint.capability, transfer)
        .unwrap();

    assert_eq!(
        kernel.network_transfer(transfer).unwrap().datagram(),
        Some(datagram)
    );
    assert_eq!(
        kernel.network_transfer(transfer).unwrap().status(),
        NetworkTransferStatus::Completed
    );
}
