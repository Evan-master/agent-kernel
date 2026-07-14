mod driver_command_support;

use agent_kernel_core::{
    DeviceEventKind, DeviceEventPayload, DriverCommandId, DriverCommandKind, DriverCommandPayload,
    DriverCommandResult, DriverCommandStatus, EventKind, Operation, OperationSet,
};

use driver_command_support::{admit_driver, prepare_bound_device, register_virtual_endpoint};

#[test]
fn driver_command_reaches_completed_with_device_event_cause() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<24, 2>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        );
    let binding = core.driver_bindings()[0].id;
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    admit_driver(&mut core, owner, driver, device);
    let cause = core
        .raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::StateChanged,
            DeviceEventPayload { code: 7, value: 9 },
        )
        .unwrap();
    let invocation = core
        .deliver_device_event(driver, driver_capability, cause)
        .unwrap();
    core.dispatch_next_driver_invocation(driver, 2).unwrap();
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
    assert_eq!(core.driver_commands()[0].invocation, Some(invocation));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.driver_commands()[0].result, None);

    let request = core
        .dispatch_driver_command(driver, driver_capability, command)
        .expect("bound driver should dispatch command");
    assert_eq!(request.command, command);
    assert_eq!(request.invocation, Some(invocation));

    let result = DriverCommandResult { code: 0, value: 12 };
    core.complete_driver_command(driver, driver_capability, command, result)
        .expect("bound driver should complete command");

    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Completed
    );
    assert_eq!(core.driver_commands()[0].result, Some(result));
    let submitted = &core.events()[core.events().len() - 3];
    assert_eq!(submitted.kind, EventKind::DriverCommandSubmitted);
    assert_eq!(submitted.driver_command, Some(command));
    assert_eq!(submitted.device_event, Some(cause));
    assert_eq!(submitted.driver_invocation, Some(invocation));
    let dispatched = &core.events()[core.events().len() - 2];
    assert_eq!(dispatched.kind, EventKind::DriverCommandDispatched);
    assert_eq!(dispatched.driver_command, Some(command));
    let completed = core.events().last().unwrap();
    assert_eq!(completed.kind, EventKind::DriverCommandCompleted);
    assert_eq!(completed.driver_command_result, Some(result));
}
