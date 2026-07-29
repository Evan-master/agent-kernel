use agent_kernel_core::{
    AgentId, KernelCore, KernelError, NetworkEndpointConfig, NetworkFrameDescriptor,
    NetworkMacAddress, NetworkTransferStatus, Operation, OperationSet, ResourceKind,
};

const OWNER: AgentId = AgentId::new(1);

fn endpoint_operations() -> OperationSet {
    OperationSet::only(Operation::Act)
        .with(Operation::Observe)
        .with(Operation::Rollback)
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

#[test]
fn endpoint_creation_is_atomic_when_the_event_log_is_full() {
    type Core = KernelCore<1, 4, 4, 4, 0, 0, 0, 0, 0, 0>;

    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let device_capability = core
        .grant_capability(OWNER, device, endpoint_operations())
        .unwrap();
    let resources_before = core.resources().len();
    let capabilities_before = core.capability_count();
    let events_before = core.events().len();

    assert_eq!(
        core.create_network_endpoint(
            OWNER,
            device_capability,
            device,
            endpoint_config(),
            endpoint_operations(),
        ),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.resources().len(), resources_before);
    assert_eq!(core.capability_count(), capabilities_before);
    assert_eq!(core.network_endpoints().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn transmit_completion_is_atomic_when_the_event_log_is_full() {
    type Core = KernelCore<1, 4, 4, 7, 0, 0, 0, 0, 0, 0>;

    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let device_capability = core
        .grant_capability(OWNER, device, endpoint_operations())
        .unwrap();
    let endpoint = core
        .create_network_endpoint(
            OWNER,
            device_capability,
            device,
            endpoint_config(),
            endpoint_operations(),
        )
        .unwrap();
    core.activate_network_endpoint(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    let transfer = core
        .prepare_network_transmit(OWNER, endpoint.capability, endpoint.resource, frame(0x11))
        .unwrap();

    assert_eq!(core.events().len(), 7);
    assert_eq!(
        core.complete_network_transmit(OWNER, endpoint.capability, transfer),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(
        core.network_transfer(transfer).unwrap().status(),
        NetworkTransferStatus::Prepared
    );
    assert_eq!(core.events().len(), 7);
}

#[test]
fn receive_ledger_rejects_overflow_without_recording_an_event() {
    type Core = KernelCore<1, 4, 2, 32, 0, 0, 0, 0, 0, 0>;

    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let device_capability = core
        .grant_capability(OWNER, device, endpoint_operations())
        .unwrap();
    let endpoint = core
        .create_network_endpoint(
            OWNER,
            device_capability,
            device,
            endpoint_config(),
            endpoint_operations(),
        )
        .unwrap();
    core.activate_network_endpoint(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    core.record_network_receive(OWNER, endpoint.capability, endpoint.resource, frame(0x21))
        .unwrap();
    core.record_network_receive(OWNER, endpoint.capability, endpoint.resource, frame(0x22))
        .unwrap();
    let events_before = core.events().len();

    assert_eq!(
        core.record_network_receive(OWNER, endpoint.capability, endpoint.resource, frame(0x23)),
        Err(KernelError::NetworkTransferStoreFull)
    );
    assert_eq!(core.network_transfers().len(), 2);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn receive_requires_observe_authority() {
    type Core = KernelCore<1, 4, 4, 16, 0, 0, 0, 0, 0, 0>;

    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let device_capability = core
        .grant_capability(OWNER, device, endpoint_operations())
        .unwrap();
    let act_only = OperationSet::only(Operation::Act).with(Operation::Rollback);
    let endpoint = core
        .create_network_endpoint(
            OWNER,
            device_capability,
            device,
            endpoint_config(),
            act_only,
        )
        .unwrap();
    core.activate_network_endpoint(OWNER, endpoint.capability, endpoint.resource)
        .unwrap();
    let events_before = core.events().len();

    assert_eq!(
        core.record_network_receive(OWNER, endpoint.capability, endpoint.resource, frame(0x31)),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.network_transfers().len(), 0);
    assert_eq!(core.events().len(), events_before);
}
