use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, NetworkEndpointConfig, NetworkEndpointStatus,
    NetworkFrameDescriptor, NetworkMacAddress, NetworkTransferDirection, NetworkTransferStatus,
    Operation, OperationSet, ResourceKind, ResourceStatus,
};

type Core = KernelCore<2, 12, 16, 64, 0, 0, 0, 0, 0, 0>;

const OWNER: AgentId = AgentId::new(1);
const OTHER: AgentId = AgentId::new(2);

fn operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

fn endpoint_config() -> NetworkEndpointConfig {
    NetworkEndpointConfig::new(
        NetworkMacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]).unwrap(),
        1500,
    )
    .unwrap()
}

fn frame(seed: u8) -> NetworkFrameDescriptor {
    NetworkFrameDescriptor::new(60, 0x0806, [seed; 32]).unwrap()
}

fn fixture() -> (
    Core,
    agent_kernel_core::ResourceId,
    agent_kernel_core::CapabilityId,
) {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    core.register_agent(OTHER).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core.grant_capability(OWNER, device, operations()).unwrap();
    (core, device, capability)
}

#[test]
fn network_endpoint_and_transfer_lifecycles_are_authorized_and_evented() {
    let (mut core, device, device_capability) = fixture();
    let first_event = core.events().len();
    let endpoint = core
        .create_network_endpoint(
            OWNER,
            device_capability,
            device,
            endpoint_config(),
            operations(),
        )
        .unwrap();
    core.activate_network_endpoint(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();

    let transmit = core
        .prepare_network_transmit(OWNER, endpoint.capability, endpoint.resource, frame(0x11))
        .unwrap();
    core.complete_network_transmit(OWNER, endpoint.capability, transmit)
        .unwrap();
    let receive = core
        .record_network_receive(OWNER, endpoint.capability, endpoint.resource, frame(0x22))
        .unwrap();

    let endpoint_record = core.network_endpoint(endpoint.resource).unwrap();
    assert_eq!(endpoint_record.device(), device);
    assert_eq!(endpoint_record.config(), endpoint_config());
    assert_eq!(endpoint_record.status(), NetworkEndpointStatus::Active);
    assert_eq!(
        core.network_transfer(transmit).unwrap().direction(),
        NetworkTransferDirection::Transmit
    );
    assert_eq!(
        core.network_transfer(transmit).unwrap().status(),
        NetworkTransferStatus::Completed
    );
    assert_eq!(
        core.network_transfer(receive).unwrap().direction(),
        NetworkTransferDirection::Receive
    );

    core.begin_network_endpoint_revoke(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    core.complete_network_endpoint_revoke(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    assert_eq!(
        core.network_endpoint(endpoint.resource).unwrap().status(),
        NetworkEndpointStatus::Released
    );
    assert_eq!(
        core.events()[first_event..]
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [
            EventKind::ResourceCreated,
            EventKind::CapabilityGranted,
            EventKind::NetworkEndpointReserved,
            EventKind::NetworkEndpointActivated,
            EventKind::NetworkTransmitPrepared,
            EventKind::NetworkTransmitCompleted,
            EventKind::NetworkReceiveRecorded,
            EventKind::NetworkEndpointRevoking,
            EventKind::NetworkEndpointReleased,
        ]
    );
}

#[test]
fn pending_transmit_blocks_revoke_until_terminal_failure_is_recorded() {
    let (mut core, device, device_capability) = fixture();
    let endpoint = core
        .create_network_endpoint(
            OWNER,
            device_capability,
            device,
            endpoint_config(),
            operations(),
        )
        .unwrap();
    core.activate_network_endpoint(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    let transmit = core
        .prepare_network_transmit(OWNER, endpoint.capability, endpoint.resource, frame(0x33))
        .unwrap();
    let event_count = core.events().len();

    assert_eq!(
        core.begin_network_endpoint_revoke(OWNER, endpoint.capability, endpoint.resource),
        Err(KernelError::NetworkTransferPending)
    );
    assert_eq!(core.events().len(), event_count);
    core.fail_network_transmit(OWNER, endpoint.capability, transmit)
        .unwrap();
    assert_eq!(
        core.network_transfer(transmit).unwrap().status(),
        NetworkTransferStatus::Failed
    );
    core.begin_network_endpoint_revoke(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
}

#[test]
fn invalid_or_unauthorized_network_operations_fail_atomically() {
    let (mut core, device, device_capability) = fixture();
    let narrow_config = NetworkEndpointConfig::new(endpoint_config().mac(), 68).unwrap();
    let endpoint = core
        .create_network_endpoint(
            OWNER,
            device_capability,
            device,
            narrow_config,
            operations(),
        )
        .unwrap();
    core.activate_network_endpoint(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    let event_count = core.events().len();
    let transfer_count = core.network_transfers().len();

    assert_eq!(
        core.prepare_network_transmit(OTHER, endpoint.capability, endpoint.resource, frame(0x44)),
        Err(KernelError::AgentMismatch)
    );
    assert_eq!(
        core.prepare_network_transmit(
            OWNER,
            endpoint.capability,
            endpoint.resource,
            NetworkFrameDescriptor::new(83, 0x0806, [0x55; 32]).unwrap(),
        ),
        Err(KernelError::NetworkFrameInvalid)
    );
    assert_eq!(core.events().len(), event_count);
    assert_eq!(core.network_transfers().len(), transfer_count);
}

#[test]
fn one_live_endpoint_per_device_and_retirement_guards_are_enforced() {
    let (mut core, device, device_capability) = fixture();
    let endpoint = core
        .create_network_endpoint(
            OWNER,
            device_capability,
            device,
            endpoint_config(),
            operations(),
        )
        .unwrap();

    assert_eq!(
        core.create_network_endpoint(
            OWNER,
            device_capability,
            device,
            endpoint_config(),
            operations(),
        ),
        Err(KernelError::NetworkEndpointAlreadyExists)
    );
    assert_eq!(
        core.retire_resource(OWNER, endpoint.capability, endpoint.resource),
        Err(KernelError::NetworkResourceBusy)
    );
    assert_eq!(
        core.retire_resource(OWNER, device_capability, device),
        Err(KernelError::NetworkResourceBusy)
    );

    core.activate_network_endpoint(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    core.begin_network_endpoint_revoke(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    core.complete_network_endpoint_revoke(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    core.retire_resource(OWNER, endpoint.capability, endpoint.resource)
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

#[test]
fn network_values_reject_noncanonical_mac_mtu_and_frame_shapes() {
    assert_eq!(NetworkMacAddress::new([0; 6]), None);
    assert_eq!(NetworkMacAddress::new([0xff; 6]), None);
    assert_eq!(NetworkMacAddress::new([0x01, 0, 0, 0, 0, 1]), None);
    let mac = NetworkMacAddress::new([0x52, 0x54, 0, 1, 2, 3]).unwrap();
    assert_eq!(
        NetworkEndpointConfig::new(mac, 67),
        Err(KernelError::NetworkEndpointInvalid)
    );
    assert_eq!(
        NetworkEndpointConfig::new(mac, 1501),
        Err(KernelError::NetworkEndpointInvalid)
    );
    assert_eq!(
        NetworkFrameDescriptor::new(13, 0x0806, [1; 32]),
        Err(KernelError::NetworkFrameInvalid)
    );
    assert_eq!(
        NetworkFrameDescriptor::new(60, 0x05ff, [1; 32]),
        Err(KernelError::NetworkFrameInvalid)
    );
}
