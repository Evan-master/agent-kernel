mod driver_command_support;

use agent_kernel_core::{
    DeviceEventKind, DeviceEventPayload, DriverCommandResult, DriverCommandStatus, KernelError,
    Operation, OperationSet,
};

use driver_command_support::{
    admit_driver, prepare_bound_device, register_virtual_endpoint, submit,
};

#[test]
fn causal_command_requires_its_invocation_to_still_be_running() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<24, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        );
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    admit_driver(&mut core, owner, driver, device);
    let event = core
        .raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::StateChanged,
            DeviceEventPayload { code: 1, value: 2 },
        )
        .unwrap();
    let invocation = core
        .deliver_device_event(driver, driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(driver, 1).unwrap();
    core.acknowledge_device_event(driver, driver_capability, event)
        .unwrap();
    let command = submit(&mut core, driver, driver_capability, device, Some(event)).unwrap();
    core.tick_driver_invocation(driver, invocation).unwrap();
    let events_before = core.events().len();

    let result = core.dispatch_driver_command(driver, driver_capability, command);

    assert_eq!(result, Err(KernelError::DriverInvocationNotRunnable));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn backend_outcome_can_arrive_after_causal_invocation_completes() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<24, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        );
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    admit_driver(&mut core, owner, driver, device);
    let event = core
        .raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::DataReady,
            DeviceEventPayload { code: 3, value: 5 },
        )
        .unwrap();
    let invocation = core
        .deliver_device_event(driver, driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(driver, 2).unwrap();
    core.acknowledge_device_event(driver, driver_capability, event)
        .unwrap();
    let command = submit(&mut core, driver, driver_capability, device, Some(event)).unwrap();
    core.dispatch_driver_command(driver, driver_capability, command)
        .unwrap();
    core.complete_driver_invocation(driver, driver_capability, invocation)
        .unwrap();

    let result = DriverCommandResult { code: 0, value: 11 };
    core.complete_driver_command(driver, driver_capability, command, result)
        .expect("terminal backend report should not require a running invocation");

    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Completed
    );
    assert_eq!(core.driver_commands()[0].result, Some(result));
}
