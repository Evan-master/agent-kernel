mod driver_command_support;

use agent_kernel_core::{
    DriverCommandResult, DriverCommandStatus, EventKind, KernelError, Operation, OperationSet,
};

use driver_command_support::{prepare_bound_device, register_virtual_endpoint, submit};

#[test]
fn submitted_command_dispatches_before_backend_completion() {
    let (mut core, owner, driver, device, owner_capability, capability) =
        prepare_bound_device::<12, 1>(
            OperationSet::only(Operation::Delegate),
            OperationSet::only(Operation::Act),
        );
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    let command = submit(&mut core, driver, capability, device, None).unwrap();

    let request = core
        .dispatch_driver_command(driver, capability, command)
        .expect("authorized command should dispatch");

    assert_eq!(request.command, command);
    assert_eq!(request.resource, device);
    assert_eq!(request.driver, driver);
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Dispatched
    );
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::DriverCommandDispatched
    );

    let result = DriverCommandResult { code: 0, value: 12 };
    core.complete_driver_command(driver, capability, command, result)
        .expect("dispatched command should accept backend result");
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Completed
    );
    assert_eq!(core.driver_commands()[0].result, Some(result));
}

#[test]
fn submitted_command_cannot_complete_before_dispatch() {
    let (mut core, _, driver, device, _, capability) = prepare_bound_device::<8, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let command = submit(&mut core, driver, capability, device, None).unwrap();
    let events_before = core.events().len();

    let result = core.complete_driver_command(
        driver,
        capability,
        command,
        DriverCommandResult { code: 0, value: 1 },
    );

    assert_eq!(result, Err(KernelError::DriverCommandStatusMismatch));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.driver_commands()[0].result, None);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn submitted_command_cannot_dispatch_without_registered_endpoint() {
    let (mut core, _, driver, device, _, capability) = prepare_bound_device::<8, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let command = submit(&mut core, driver, capability, device, None).unwrap();
    let events_before = core.events().len();

    let result = core.dispatch_driver_command(driver, capability, command);

    assert_eq!(result, Err(KernelError::DriverEndpointNotFound));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.events().len(), events_before);
}
