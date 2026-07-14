mod driver_command_support;

use agent_kernel_core::{
    DriverCommandId, DriverCommandResult, DriverCommandStatus, EventKind, KernelError, Operation,
    OperationSet,
};

use driver_command_support::{prepare_bound_device, register_virtual_endpoint, submit};

#[test]
fn complete_driver_command_requires_bound_driver_without_mutation() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<9, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::only(Operation::Act),
        );
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    let command = submit(&mut core, driver, driver_capability, device, None).unwrap();
    core.dispatch_driver_command(driver, driver_capability, command)
        .unwrap();
    let events_before = core.events().len();

    let result = core.complete_driver_command(
        owner,
        owner_capability,
        command,
        DriverCommandResult { code: 0, value: 1 },
    );

    assert_eq!(result, Err(KernelError::AgentMismatch));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Dispatched
    );
    assert_eq!(core.driver_commands()[0].result, None);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn complete_driver_command_requires_act_authority_without_mutation() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<10, 1>(
            OperationSet::only(Operation::Delegate),
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        );
    let observe_capability = core
        .grant_capability(driver, device, OperationSet::only(Operation::Observe))
        .unwrap();
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    let command = submit(&mut core, driver, driver_capability, device, None).unwrap();
    core.dispatch_driver_command(driver, driver_capability, command)
        .unwrap();
    let events_before = core.events().len();

    let result = core.complete_driver_command(
        driver,
        observe_capability,
        command,
        DriverCommandResult { code: 0, value: 1 },
    );

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Dispatched
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn complete_driver_command_log_full_leaves_command_dispatched() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<8, 1>(
            OperationSet::only(Operation::Delegate),
            OperationSet::only(Operation::Act),
        );
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    let command = submit(&mut core, driver, driver_capability, device, None).unwrap();
    core.dispatch_driver_command(driver, driver_capability, command)
        .unwrap();

    let result = core.complete_driver_command(
        driver,
        driver_capability,
        command,
        DriverCommandResult { code: 0, value: 1 },
    );

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Dispatched
    );
    assert_eq!(core.driver_commands()[0].result, None);
    assert_eq!(core.events().len(), 8);
}

#[test]
fn fail_driver_command_records_terminal_failure() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<9, 1>(
            OperationSet::only(Operation::Delegate),
            OperationSet::only(Operation::Act),
        );
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    let command = submit(&mut core, driver, driver_capability, device, None).unwrap();
    core.dispatch_driver_command(driver, driver_capability, command)
        .unwrap();
    let failure = DriverCommandResult { code: 5, value: 13 };

    core.fail_driver_command(driver, driver_capability, command, failure)
        .unwrap();

    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Failed
    );
    assert_eq!(core.driver_commands()[0].result, Some(failure));
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::DriverCommandFailed
    );
}

#[test]
fn terminal_driver_command_rejects_second_transition_without_mutation() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<9, 1>(
            OperationSet::only(Operation::Delegate),
            OperationSet::only(Operation::Act),
        );
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    let command = submit(&mut core, driver, driver_capability, device, None).unwrap();
    core.dispatch_driver_command(driver, driver_capability, command)
        .unwrap();
    let completed = DriverCommandResult { code: 0, value: 1 };
    core.complete_driver_command(driver, driver_capability, command, completed)
        .unwrap();
    let events_before = core.events().len();

    let result = core.fail_driver_command(
        driver,
        driver_capability,
        command,
        DriverCommandResult { code: 2, value: 3 },
    );

    assert_eq!(result, Err(KernelError::DriverCommandStatusMismatch));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Completed
    );
    assert_eq!(core.driver_commands()[0].result, Some(completed));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn retired_resource_rejects_driver_command_transition() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<10, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Rollback),
            OperationSet::only(Operation::Act),
        );
    register_virtual_endpoint(&mut core, owner, owner_capability, device);
    let command = submit(&mut core, driver, driver_capability, device, None).unwrap();
    core.dispatch_driver_command(driver, driver_capability, command)
        .unwrap();
    core.retire_resource(owner, owner_capability, device)
        .unwrap();
    let events_before = core.events().len();

    let result = core.complete_driver_command(
        driver,
        driver_capability,
        command,
        DriverCommandResult { code: 0, value: 1 },
    );

    assert_eq!(result, Err(KernelError::ResourceRetired));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Dispatched
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn missing_driver_command_transition_is_rejected_without_event() {
    let (mut core, _, driver, _, _, driver_capability) = prepare_bound_device::<8, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let events_before = core.events().len();

    let result = core.complete_driver_command(
        driver,
        driver_capability,
        DriverCommandId::new(99),
        DriverCommandResult { code: 0, value: 1 },
    );

    assert_eq!(result, Err(KernelError::DriverCommandNotFound));
    assert_eq!(core.events().len(), events_before);
}
