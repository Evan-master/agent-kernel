mod driver_command_support;

use agent_kernel_core::{
    DriverCommandId, DriverCommandStatus, KernelError, Operation, OperationSet,
};

use driver_command_support::{prepare_bound_device, submit};

#[test]
fn missing_command_cannot_dispatch() {
    let (mut core, _, driver, _, _, capability) = prepare_bound_device::<8, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let events_before = core.events().len();

    let result = core.dispatch_driver_command(driver, capability, DriverCommandId::new(99));

    assert_eq!(result, Err(KernelError::DriverCommandNotFound));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn only_bound_driver_can_dispatch() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<8, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::only(Operation::Act),
        );
    let command = submit(&mut core, driver, driver_capability, device, None).unwrap();
    let events_before = core.events().len();

    let result = core.dispatch_driver_command(owner, owner_capability, command);

    assert_eq!(result, Err(KernelError::AgentMismatch));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn revoked_act_authority_blocks_dispatch() {
    let (mut core, _, driver, device, _, capability) = prepare_bound_device::<9, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let command = submit(&mut core, driver, capability, device, None).unwrap();
    core.revoke_capability(capability).unwrap();
    let events_before = core.events().len();

    let result = core.dispatch_driver_command(driver, capability, command);

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn retired_resource_blocks_dispatch() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<9, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Rollback),
            OperationSet::only(Operation::Act),
        );
    let command = submit(&mut core, driver, driver_capability, device, None).unwrap();
    core.retire_resource(owner, owner_capability, device)
        .unwrap();
    let events_before = core.events().len();

    let result = core.dispatch_driver_command(driver, driver_capability, command);

    assert_eq!(result, Err(KernelError::ResourceRetired));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn full_event_log_leaves_command_submitted() {
    let (mut core, _, driver, device, _, capability) = prepare_bound_device::<6, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let command = submit(&mut core, driver, capability, device, None).unwrap();

    let result = core.dispatch_driver_command(driver, capability, command);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Submitted
    );
    assert_eq!(core.driver_commands()[0].result, None);
    assert_eq!(core.events().len(), 6);
}

#[test]
fn command_cannot_dispatch_twice() {
    let (mut core, _, driver, device, _, capability) = prepare_bound_device::<8, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let command = submit(&mut core, driver, capability, device, None).unwrap();
    core.dispatch_driver_command(driver, capability, command)
        .unwrap();
    let events_before = core.events().len();

    let result = core.dispatch_driver_command(driver, capability, command);

    assert_eq!(result, Err(KernelError::DriverCommandStatusMismatch));
    assert_eq!(
        core.driver_commands()[0].status,
        DriverCommandStatus::Dispatched
    );
    assert_eq!(core.events().len(), events_before);
}
