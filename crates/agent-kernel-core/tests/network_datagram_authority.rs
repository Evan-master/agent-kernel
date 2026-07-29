use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, NetworkDatagramDescriptor, NetworkEndpointConfig,
    NetworkFrameDescriptor, NetworkIpv4Address, NetworkMacAddress, NetworkTransferDirection,
    NetworkTransferStatus, NetworkUdpPort, Operation, OperationSet, ResourceKind,
};

type Core = KernelCore<1, 8, 12, 32, 0, 0, 0, 0, 0, 0>;

const AGENT: AgentId = AgentId::new(1);

fn ipv4(bytes: [u8; 4]) -> NetworkIpv4Address {
    NetworkIpv4Address::new(bytes).unwrap()
}

fn port(value: u16) -> NetworkUdpPort {
    NetworkUdpPort::new(value).unwrap()
}

fn outbound_datagram(payload_length: u16, seed: u8) -> NetworkDatagramDescriptor {
    NetworkDatagramDescriptor::new(
        ipv4([10, 0, 2, 15]),
        ipv4([10, 0, 2, 2]),
        port(40131),
        port(40130),
        payload_length,
        [seed; 32],
    )
    .unwrap()
}

fn inbound_datagram(payload_length: u16, seed: u8) -> NetworkDatagramDescriptor {
    NetworkDatagramDescriptor::new(
        ipv4([10, 0, 2, 2]),
        ipv4([10, 0, 2, 15]),
        port(40130),
        port(40131),
        payload_length,
        [seed; 32],
    )
    .unwrap()
}

fn ipv4_frame(payload_length: u16, seed: u8) -> NetworkFrameDescriptor {
    let wire_length = (14 + 20 + 8 + payload_length).max(60);
    NetworkFrameDescriptor::new(wire_length, 0x0800, [seed; 32]).unwrap()
}

fn fixture() -> (
    Core,
    agent_kernel_core::ResourceId,
    agent_kernel_core::CapabilityId,
) {
    let mut core = Core::new();
    core.register_agent(AGENT).unwrap();
    let operations = OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let device_capability = core.grant_capability(AGENT, device, operations).unwrap();
    let endpoint = core
        .create_network_endpoint(
            AGENT,
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
    core.activate_network_endpoint(AGENT, endpoint.capability, endpoint.resource)
        .unwrap();
    (core, endpoint.resource, endpoint.capability)
}

#[test]
fn ipv4_udp_transfers_are_authorized_and_retain_semantic_evidence() {
    let (mut core, endpoint, capability) = fixture();
    let first_event = core.events().len();
    let outbound = outbound_datagram(13, 0x11);
    let transmit = core
        .prepare_network_datagram_transmit(
            AGENT,
            capability,
            endpoint,
            ipv4_frame(13, 0x21),
            outbound,
        )
        .unwrap();
    core.complete_network_transmit(AGENT, capability, transmit)
        .unwrap();
    let inbound = inbound_datagram(13, 0x11);
    let receive = core
        .record_network_datagram_receive(AGENT, capability, endpoint, ipv4_frame(13, 0x31), inbound)
        .unwrap();

    let transmitted = core.network_transfer(transmit).unwrap();
    assert_eq!(transmitted.direction(), NetworkTransferDirection::Transmit);
    assert_eq!(transmitted.status(), NetworkTransferStatus::Completed);
    assert_eq!(transmitted.datagram(), Some(outbound));
    assert_eq!(
        core.network_transfer(receive).unwrap().datagram(),
        Some(inbound)
    );
    assert_eq!(
        core.events()[first_event..]
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [
            EventKind::NetworkDatagramTransmitPrepared,
            EventKind::NetworkDatagramTransmitCompleted,
            EventKind::NetworkDatagramReceiveRecorded,
        ]
    );
}

#[test]
fn datagram_and_frame_contract_mismatches_fail_atomically() {
    let (mut core, endpoint, capability) = fixture();
    let event_count = core.events().len();
    let transfer_count = core.network_transfers().len();
    let datagram = outbound_datagram(13, 0x41);

    assert_eq!(
        core.prepare_network_datagram_transmit(
            AGENT,
            capability,
            endpoint,
            NetworkFrameDescriptor::new(60, 0x0806, [0x51; 32]).unwrap(),
            datagram,
        ),
        Err(KernelError::NetworkDatagramFrameMismatch)
    );
    assert_eq!(
        core.record_network_datagram_receive(
            AGENT,
            capability,
            endpoint,
            NetworkFrameDescriptor::new(61, 0x0800, [0x61; 32]).unwrap(),
            inbound_datagram(13, 0x41),
        ),
        Err(KernelError::NetworkDatagramFrameMismatch)
    );
    assert_eq!(core.events().len(), event_count);
    assert_eq!(core.network_transfers().len(), transfer_count);
}

#[test]
fn ipv4_udp_values_reject_noncanonical_addresses_ports_and_lengths() {
    assert_eq!(NetworkIpv4Address::new([0, 0, 0, 0]), None);
    assert_eq!(NetworkIpv4Address::new([127, 0, 0, 1]), None);
    assert_eq!(NetworkIpv4Address::new([224, 0, 0, 1]), None);
    assert_eq!(NetworkIpv4Address::new([255, 255, 255, 255]), None);
    assert_eq!(NetworkUdpPort::new(0), None);
    assert_eq!(
        NetworkDatagramDescriptor::new(
            ipv4([10, 0, 2, 15]),
            ipv4([10, 0, 2, 2]),
            port(40131),
            port(40130),
            1473,
            [0; 32],
        ),
        Err(KernelError::NetworkDatagramInvalid)
    );
}
