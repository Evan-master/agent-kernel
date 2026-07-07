use agent_kernel_core::{
    AgentId, DriverBindingId, EventKind, KernelCore, Operation, OperationSet, ResourceKind,
};

type TestKernel = KernelCore<4, 4, 4, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0>;

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
