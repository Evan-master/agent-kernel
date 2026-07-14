use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind,
    DeviceEventKind, DeviceEventPayload, DeviceEventStatus, DriverCommandKind,
    DriverCommandPayload, DriverCommandResult, DriverInvocationId, DriverInvocationStatus,
    EventKind, KernelCore, Operation, OperationSet, ResourceKind,
};

type TestKernel = KernelCore<4, 4, 10, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2>;

#[test]
fn delivered_event_runs_as_driver_invocation_and_completes() {
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
                .with(Operation::Act)
                .with(Operation::Verify),
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
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            device,
            AgentImageKind::Driver,
            AgentImageDigest::new([3; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(owner, owner_capability, image)
        .unwrap();
    core.launch_agent(
        driver,
        driver_capability,
        device,
        image,
        AgentEntryKind::Driver,
        None,
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

    let invocation = core
        .deliver_device_event(driver, driver_capability, event)
        .expect("delivery should queue driver invocation");

    assert_eq!(invocation, DriverInvocationId::new(1));
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Delivered);
    assert_eq!(core.driver_invocations().len(), 1);
    assert_eq!(core.driver_invocations()[0].binding, binding);
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Queued
    );
    assert_eq!(
        core.events()[core.events().len() - 2].kind,
        EventKind::DeviceEventDelivered
    );
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::DriverInvocationQueued
    );

    let dispatched = core
        .dispatch_next_driver_invocation(driver, 2)
        .expect("queued driver invocation should dispatch");
    assert_eq!(dispatched, invocation);
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Running
    );
    let context = core.execution_context(driver).unwrap();
    assert_eq!(context.state, AgentExecutionState::Running);
    assert_eq!(context.task, None);
    assert_eq!(context.driver_invocation, Some(invocation));

    core.acknowledge_device_event(driver, driver_capability, event)
        .expect("running driver should acknowledge event");
    let command = core
        .submit_driver_command(
            driver,
            driver_capability,
            device,
            Some(event),
            DriverCommandKind::Write,
            DriverCommandPayload {
                opcode: 3,
                value: 11,
            },
        )
        .expect("running driver should submit response command");
    assert_eq!(core.driver_commands()[0].invocation, Some(invocation));
    core.dispatch_driver_command(driver, driver_capability, command)
        .unwrap();
    core.complete_driver_command(
        driver,
        driver_capability,
        command,
        DriverCommandResult { code: 0, value: 12 },
    )
    .unwrap();
    core.complete_driver_invocation(driver, driver_capability, invocation)
        .expect("acknowledged invocation should complete");

    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Completed
    );
    assert_eq!(
        core.execution_context(driver).unwrap().state,
        AgentExecutionState::Idle
    );
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::DriverInvocationCompleted
    );
}
