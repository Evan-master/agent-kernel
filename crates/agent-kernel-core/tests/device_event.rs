use agent_kernel_core::{
    AgentId, CapabilityId, DeviceEventId, DeviceEventKind, DeviceEventPayload, DeviceEventStatus,
    EventKind, KernelCore, KernelError, Operation, OperationSet, ResourceId, ResourceKind,
};

type TestKernel = KernelCore<4, 4, 6, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2>;
type EventKernel<const EVENTS: usize, const DEVICE_EVENTS: usize> =
    KernelCore<4, 4, 8, EVENTS, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, DEVICE_EVENTS>;

#[test]
fn device_event_reaches_acknowledged_through_bound_driver() {
    let mut core = TestKernel::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let owner_capability = core
        .grant_capability(
            owner,
            device,
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
        )
        .unwrap();
    let driver_capability = core
        .grant_capability(
            driver,
            device,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .unwrap();
    let binding = core
        .bind_driver(owner, owner_capability, device, driver)
        .unwrap();

    let event = core
        .raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::StateChanged,
            DeviceEventPayload { code: 7, value: 9 },
        )
        .unwrap();
    assert_eq!(event, DeviceEventId::new(1));
    assert_eq!(core.device_events()[0].binding, binding);
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);

    core.deliver_device_event(driver, driver_capability, event)
        .unwrap();
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Delivered);
    core.acknowledge_device_event(driver, driver_capability, event)
        .unwrap();
    assert_eq!(
        core.device_events()[0].status,
        DeviceEventStatus::Acknowledged
    );

    let kinds: [EventKind; 4] = [
        core.events()[4].kind,
        core.events()[5].kind,
        core.events()[6].kind,
        core.events()[7].kind,
    ];
    assert_eq!(
        kinds,
        [
            EventKind::DriverBound,
            EventKind::DeviceEventRaised,
            EventKind::DeviceEventDelivered,
            EventKind::DeviceEventAcknowledged,
        ]
    );
}

#[test]
fn raise_device_event_requires_act_authority_without_mutation() {
    let (mut core, owner, _, device, owner_capability, _) = prepare_bound_device::<12, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Observe),
    );
    let events_before = core.events().len();

    let result = core.raise_device_event(
        owner,
        owner_capability,
        device,
        DeviceEventKind::StateChanged,
        DeviceEventPayload { code: 1, value: 2 },
    );

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.device_events().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn raise_device_event_requires_existing_binding_without_mutation() {
    let mut core = EventKernel::<12, 1>::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core
        .grant_capability(owner, device, OperationSet::only(Operation::Act))
        .unwrap();
    let events_before = core.events().len();

    let result = core.raise_device_event(
        owner,
        capability,
        device,
        DeviceEventKind::DataReady,
        DeviceEventPayload { code: 3, value: 4 },
    );

    assert_eq!(result, Err(KernelError::DriverBindingNotFound));
    assert_eq!(core.device_events().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn raise_device_event_store_full_leaves_event_log_unchanged() {
    let (mut core, owner, _, device, owner_capability, _) = prepare_bound_device::<12, 0>(
        OperationSet::empty()
            .with(Operation::Delegate)
            .with(Operation::Act),
        OperationSet::only(Operation::Observe),
    );
    let events_before = core.events().len();

    let result = core.raise_device_event(
        owner,
        owner_capability,
        device,
        DeviceEventKind::Fault,
        DeviceEventPayload { code: 5, value: 6 },
    );

    assert_eq!(result, Err(KernelError::DeviceEventStoreFull));
    assert_eq!(core.device_events().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn raise_device_event_log_full_leaves_no_device_event() {
    let (mut core, owner, _, device, owner_capability, _) = prepare_bound_device::<5, 1>(
        OperationSet::empty()
            .with(Operation::Delegate)
            .with(Operation::Act),
        OperationSet::only(Operation::Observe),
    );

    let result = core.raise_device_event(
        owner,
        owner_capability,
        device,
        DeviceEventKind::Interrupt,
        DeviceEventPayload { code: 7, value: 8 },
    );

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.device_events().len(), 0);
    assert_eq!(core.events().len(), 5);
}

#[test]
fn deliver_device_event_requires_bound_driver_without_mutation() {
    let (mut core, owner, driver, device, owner_capability, _) = prepare_bound_device::<12, 1>(
        OperationSet::empty()
            .with(Operation::Delegate)
            .with(Operation::Act)
            .with(Operation::Observe),
        OperationSet::empty()
            .with(Operation::Observe)
            .with(Operation::Act),
    );
    let event = raise_state_event(&mut core, owner, owner_capability, device);
    let events_before = core.events().len();

    let result = core.deliver_device_event(owner, owner_capability, event);

    assert_eq!(result, Err(KernelError::AgentMismatch));
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);
    assert_eq!(core.events().len(), events_before);
    assert_eq!(core.driver_bindings()[0].driver, driver);
}

#[test]
fn deliver_device_event_requires_observe_authority_without_mutation() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<12, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::only(Operation::Act),
        );
    let event = raise_state_event(&mut core, owner, owner_capability, device);
    let events_before = core.events().len();

    let result = core.deliver_device_event(driver, driver_capability, event);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn acknowledge_device_event_requires_act_authority_without_mutation() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<12, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::only(Operation::Observe),
        );
    let event = raise_state_event(&mut core, owner, owner_capability, device);
    core.deliver_device_event(driver, driver_capability, event)
        .unwrap();
    let events_before = core.events().len();

    let result = core.acknowledge_device_event(driver, driver_capability, event);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Delivered);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn repeated_delivery_or_acknowledgement_is_rejected_without_mutation() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<12, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        );
    let event = raise_state_event(&mut core, owner, owner_capability, device);
    core.deliver_device_event(driver, driver_capability, event)
        .unwrap();
    let delivered_events = core.events().len();

    let second_delivery = core.deliver_device_event(driver, driver_capability, event);

    assert_eq!(second_delivery, Err(KernelError::DeviceEventStatusMismatch));
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Delivered);
    assert_eq!(core.events().len(), delivered_events);

    core.acknowledge_device_event(driver, driver_capability, event)
        .unwrap();
    let acknowledged_events = core.events().len();

    let second_acknowledgement = core.acknowledge_device_event(driver, driver_capability, event);

    assert_eq!(
        second_acknowledgement,
        Err(KernelError::DeviceEventStatusMismatch)
    );
    assert_eq!(
        core.device_events()[0].status,
        DeviceEventStatus::Acknowledged
    );
    assert_eq!(core.events().len(), acknowledged_events);
}

#[test]
fn retired_device_resource_rejects_event_transitions() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<12, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act)
                .with(Operation::Rollback),
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        );
    let event = raise_state_event(&mut core, owner, owner_capability, device);
    core.retire_resource(owner, owner_capability, device)
        .unwrap();
    let events_before = core.events().len();

    let result = core.deliver_device_event(driver, driver_capability, event);

    assert_eq!(result, Err(KernelError::ResourceRetired));
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);
    assert_eq!(core.events().len(), events_before);
}

fn prepare_bound_device<const EVENTS: usize, const DEVICE_EVENTS: usize>(
    owner_operations: OperationSet,
    driver_operations: OperationSet,
) -> (
    EventKernel<EVENTS, DEVICE_EVENTS>,
    AgentId,
    AgentId,
    ResourceId,
    CapabilityId,
    CapabilityId,
) {
    let mut core = EventKernel::<EVENTS, DEVICE_EVENTS>::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let owner_capability = core
        .grant_capability(owner, device, owner_operations)
        .unwrap();
    let driver_capability = core
        .grant_capability(driver, device, driver_operations)
        .unwrap();
    core.bind_driver(owner, owner_capability, device, driver)
        .unwrap();

    (
        core,
        owner,
        driver,
        device,
        owner_capability,
        driver_capability,
    )
}

fn raise_state_event<const EVENTS: usize, const DEVICE_EVENTS: usize>(
    core: &mut EventKernel<EVENTS, DEVICE_EVENTS>,
    owner: AgentId,
    capability: CapabilityId,
    device: ResourceId,
) -> DeviceEventId {
    core.raise_device_event(
        owner,
        capability,
        device,
        DeviceEventKind::StateChanged,
        DeviceEventPayload { code: 7, value: 9 },
    )
    .unwrap()
}
