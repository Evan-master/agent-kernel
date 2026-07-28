use agent_kernel_core::{
    AgentId, EventKind, InterruptMode, InterruptRouteStatus, InterruptTarget, KernelCore,
    KernelError, Operation, OperationSet, ResourceKind, ResourceStatus,
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

#[test]
fn interrupt_route_lifecycle_is_capability_checked_and_evented() {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    core.register_agent(OTHER).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let device_capability = core.grant_capability(OWNER, device, operations()).unwrap();
    let first_event = core.events().len();
    let target = InterruptTarget::new(0, 0xd0).unwrap();

    let route = core
        .create_interrupt_route(
            OWNER,
            device_capability,
            device,
            InterruptMode::Msi,
            target,
            operations(),
        )
        .unwrap();
    core.activate_interrupt_route(OWNER, route.capability, route.resource)
        .unwrap();
    core.begin_interrupt_route_revoke(OWNER, route.capability, route.resource)
        .unwrap();
    core.complete_interrupt_route_revoke(OWNER, route.capability, route.resource)
        .unwrap();

    let record = core.interrupt_route(route.resource).unwrap();
    assert_eq!(record.device(), device);
    assert_eq!(record.mode(), InterruptMode::Msi);
    assert_eq!(record.target(), target);
    assert_eq!(record.status(), InterruptRouteStatus::Released);
    assert_eq!(
        core.events()[first_event..]
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [
            EventKind::ResourceCreated,
            EventKind::CapabilityGranted,
            EventKind::InterruptRouteReserved,
            EventKind::InterruptRouteActivated,
            EventKind::InterruptRouteRevoking,
            EventKind::InterruptRouteReleased,
        ]
    );
}

#[test]
fn duplicate_route_keys_and_live_vectors_fail_atomically() {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    core.register_agent(OTHER).unwrap();
    let first_device = core.register_resource(ResourceKind::Device, None).unwrap();
    let second_device = core.register_resource(ResourceKind::Device, None).unwrap();
    let first_capability = core
        .grant_capability(OWNER, first_device, operations())
        .unwrap();
    let second_capability = core
        .grant_capability(OWNER, second_device, operations())
        .unwrap();
    let first = core
        .create_interrupt_route(
            OWNER,
            first_capability,
            first_device,
            InterruptMode::Msi,
            InterruptTarget::new(0, 0xd0).unwrap(),
            operations(),
        )
        .unwrap();
    let event_count = core.events().len();
    let resource_count = core.resources().len();

    assert_eq!(
        core.create_interrupt_route(
            OWNER,
            first_capability,
            first_device,
            InterruptMode::Msi,
            InterruptTarget::new(0, 0xd2).unwrap(),
            operations(),
        ),
        Err(KernelError::InterruptRouteAlreadyExists)
    );
    assert_eq!(
        core.create_interrupt_route(
            OWNER,
            second_capability,
            second_device,
            InterruptMode::MsiX { table_entry: 0 },
            InterruptTarget::new(0, 0xd0).unwrap(),
            operations(),
        ),
        Err(KernelError::InterruptVectorInUse)
    );
    assert_eq!(
        core.activate_interrupt_route(OTHER, first.capability, first.resource),
        Err(KernelError::AgentMismatch)
    );
    assert_eq!(core.events().len(), event_count);
    assert_eq!(core.resources().len(), resource_count);
    assert_eq!(
        core.interrupt_route(first.resource).unwrap().status(),
        InterruptRouteStatus::Reserved
    );
}

#[test]
fn msix_routes_allow_distinct_entries_and_reuse_released_vectors() {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core.grant_capability(OWNER, device, operations()).unwrap();
    let first = core
        .create_interrupt_route(
            OWNER,
            capability,
            device,
            InterruptMode::MsiX { table_entry: 0 },
            InterruptTarget::new(0, 0xd0).unwrap(),
            operations(),
        )
        .unwrap();
    let second = core
        .create_interrupt_route(
            OWNER,
            capability,
            device,
            InterruptMode::MsiX { table_entry: 1 },
            InterruptTarget::new(0, 0xd1).unwrap(),
            operations(),
        )
        .unwrap();
    assert_ne!(first.resource, second.resource);

    core.activate_interrupt_route(OWNER, first.capability, first.resource)
        .unwrap();
    core.begin_interrupt_route_revoke(OWNER, first.capability, first.resource)
        .unwrap();
    core.complete_interrupt_route_revoke(OWNER, first.capability, first.resource)
        .unwrap();
    core.create_interrupt_route(
        OWNER,
        capability,
        device,
        InterruptMode::MsiX { table_entry: 2 },
        InterruptTarget::new(0, 0xd0).unwrap(),
        operations(),
    )
    .expect("released vectors can be assigned to a new route");
}

#[test]
fn live_interrupt_routes_block_route_and_device_retirement() {
    let mut core = Core::new();
    core.register_agent(OWNER).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core.grant_capability(OWNER, device, operations()).unwrap();
    let route = core
        .create_interrupt_route(
            OWNER,
            capability,
            device,
            InterruptMode::Msi,
            InterruptTarget::new(0, 0xd0).unwrap(),
            operations(),
        )
        .unwrap();

    assert_eq!(
        core.retire_resource(OWNER, route.capability, route.resource),
        Err(KernelError::InterruptResourceBusy)
    );
    assert_eq!(
        core.retire_resource(OWNER, capability, device),
        Err(KernelError::InterruptResourceBusy)
    );

    core.activate_interrupt_route(OWNER, route.capability, route.resource)
        .unwrap();
    core.begin_interrupt_route_revoke(OWNER, route.capability, route.resource)
        .unwrap();
    core.complete_interrupt_route_revoke(OWNER, route.capability, route.resource)
        .unwrap();
    core.retire_resource(OWNER, route.capability, route.resource)
        .unwrap();
    core.retire_resource(OWNER, capability, device).unwrap();
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
fn interrupt_targets_reject_exception_and_kernel_reserved_vectors() {
    assert_eq!(InterruptTarget::new(0, 0x1f), None);
    assert_eq!(InterruptTarget::new(0, 0xe0), None);
    assert!(InterruptTarget::new(u32::MAX, 0xdf).is_some());
}
