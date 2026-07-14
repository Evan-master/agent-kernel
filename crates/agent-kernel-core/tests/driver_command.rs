use agent_kernel_core::{
    AgentId, DeviceEventKind, DeviceEventPayload, DriverCommandId, DriverCommandKind,
    DriverCommandPayload, DriverCommandResult, DriverCommandStatus, EventKind, KernelCore,
    Operation, OperationSet, ResourceKind,
};

type TestKernel = KernelCore<4, 4, 8, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2>;

#[test]
fn driver_command_reaches_completed_with_device_event_cause() {
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
    let cause = core
        .raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::StateChanged,
            DeviceEventPayload { code: 7, value: 9 },
        )
        .unwrap();
    core.deliver_device_event(driver, driver_capability, cause)
        .unwrap();
    core.acknowledge_device_event(driver, driver_capability, cause)
        .unwrap();

    let command = core
        .submit_driver_command(
            driver,
            driver_capability,
            device,
            Some(cause),
            DriverCommandKind::Write,
            DriverCommandPayload {
                opcode: 3,
                value: 11,
            },
        )
        .expect("bound driver should submit command");

    assert_eq!(command, DriverCommandId::new(1));
    assert_eq!(core.driver_commands().len(), 1);
    assert_eq!(core.driver_commands()[0].binding, binding);
    assert_eq!(core.driver_commands()[0].driver, driver);
    assert_eq!(core.driver_commands()[0].cause, Some(cause));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.driver_commands()[0].result, None);

    let result = DriverCommandResult { code: 0, value: 12 };
    core.complete_driver_command(driver, driver_capability, command, result)
        .expect("bound driver should complete command");

    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Completed
    );
    assert_eq!(core.driver_commands()[0].result, Some(result));
    let submitted = &core.events()[core.events().len() - 2];
    assert_eq!(submitted.kind, EventKind::DriverCommandSubmitted);
    assert_eq!(submitted.driver_command, Some(command));
    assert_eq!(submitted.device_event, Some(cause));
    let completed = core.events().last().unwrap();
    assert_eq!(completed.kind, EventKind::DriverCommandCompleted);
    assert_eq!(completed.driver_command_result, Some(result));
}
