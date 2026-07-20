use agent_kernel_core::{
    AgentId, CapabilityId, DriverBindingId, EventKind, KernelCore, KernelError, Operation,
    OperationSet, ResourceId, ResourceKind,
};

type TestKernel = KernelCore<4, 4, 4, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0>;
type BindingKernel<const EVENTS: usize, const DRIVER_BINDINGS: usize> =
    KernelCore<4, 4, 4, EVENTS, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, DRIVER_BINDINGS, 0>;

#[test]
fn bind_driver_records_binding_and_event() {
    let mut core = TestKernel::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);

    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core
        .grant_capability(owner, device, OperationSet::only(Operation::Delegate))
        .unwrap();

    let binding = core
        .bind_driver(owner, capability, device, driver)
        .expect("driver should bind");

    assert_eq!(binding, DriverBindingId::new(1));
    assert_eq!(core.driver_bindings().len(), 1);
    assert_eq!(core.driver_bindings()[0].driver, driver);
    assert_eq!(core.driver_bindings()[0].resource, device);
    let event = core.events().last().unwrap();
    assert_eq!(event.kind, EventKind::DriverBound);
    assert_eq!(event.driver_binding, Some(binding));
    assert_eq!(event.target_agent, Some(driver));
}

#[test]
fn bind_driver_requires_delegate_authority_without_mutation() {
    let (mut core, owner, driver, device, capability) =
        prepare_device::<8, 1>(OperationSet::only(Operation::Act));
    let events_before = core.events().len();

    let result = core.bind_driver(owner, capability, device, driver);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.driver_bindings().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn bind_driver_rejects_inactive_driver_without_allocation() {
    let mut core = BindingKernel::<8, 1>::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core
        .grant_capability(owner, device, OperationSet::only(Operation::Delegate))
        .unwrap();
    let events_before = core.events().len();

    let result = core.bind_driver(owner, capability, device, driver);

    assert_eq!(result, Err(KernelError::AgentNotFound));
    assert_eq!(core.driver_bindings().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn bind_driver_accepts_workspace_control_plane_resource() {
    let mut core = BindingKernel::<8, 1>::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let capability = core
        .grant_capability(owner, workspace, OperationSet::only(Operation::Delegate))
        .unwrap();
    let binding = core
        .bind_driver(owner, capability, workspace, driver)
        .expect("Workspace control plane should accept an explicit driver binding");

    assert_eq!(binding, DriverBindingId::new(1));
    assert_eq!(core.driver_bindings()[0].resource, workspace);
    assert_eq!(
        core.driver_bindings()[0].resource_kind,
        ResourceKind::Workspace
    );
}

#[test]
fn bind_driver_rejects_non_driver_resource_without_mutation() {
    let mut core = BindingKernel::<8, 1>::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let memory = core.register_resource(ResourceKind::Memory, None).unwrap();
    let capability = core
        .grant_capability(owner, memory, OperationSet::only(Operation::Delegate))
        .unwrap();
    let events_before = core.events().len();

    let result = core.bind_driver(owner, capability, memory, driver);

    assert_eq!(result, Err(KernelError::ResourceKindMismatch));
    assert!(core.driver_bindings().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn bind_driver_rejects_duplicate_without_second_event() {
    let (mut core, owner, driver, device, capability) =
        prepare_device::<8, 1>(OperationSet::only(Operation::Delegate));
    let binding = core.bind_driver(owner, capability, device, driver).unwrap();
    let events_before = core.events().len();

    let result = core.bind_driver(owner, capability, device, driver);

    assert_eq!(result, Err(KernelError::DriverBindingAlreadyExists));
    assert_eq!(core.driver_bindings().len(), 1);
    assert_eq!(core.driver_bindings()[0].id, binding);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn bind_driver_store_full_leaves_event_log_unchanged() {
    let (mut core, owner, driver, device, capability) =
        prepare_device::<8, 0>(OperationSet::only(Operation::Delegate));
    let events_before = core.events().len();

    let result = core.bind_driver(owner, capability, device, driver);

    assert_eq!(result, Err(KernelError::DriverBindingStoreFull));
    assert_eq!(core.driver_bindings().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn bind_driver_event_log_full_leaves_no_binding() {
    let (mut core, owner, driver, device, capability) =
        prepare_device::<3, 1>(OperationSet::only(Operation::Delegate));

    let result = core.bind_driver(owner, capability, device, driver);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.driver_bindings().len(), 0);
    assert_eq!(core.events().len(), 3);
}

fn prepare_device<const EVENTS: usize, const DRIVER_BINDINGS: usize>(
    operations: OperationSet,
) -> (
    BindingKernel<EVENTS, DRIVER_BINDINGS>,
    AgentId,
    AgentId,
    ResourceId,
    CapabilityId,
) {
    let mut core = BindingKernel::<EVENTS, DRIVER_BINDINGS>::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core.grant_capability(owner, device, operations).unwrap();

    (core, owner, driver, device, capability)
}
