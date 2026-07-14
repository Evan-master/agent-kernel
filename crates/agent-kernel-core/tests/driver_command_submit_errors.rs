mod driver_command_support;

use agent_kernel_core::{
    AgentId, DeviceEventId, DeviceEventKind, DeviceEventPayload, DriverCommandId, KernelError,
    Operation, OperationSet, ResourceKind,
};

use driver_command_support::{prepare_bound_device, submit, EventKernel};

#[test]
fn submit_driver_command_requires_existing_binding_without_mutation() {
    let mut core = EventKernel::<8, 1>::new();
    let driver = AgentId::new(1);
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let capability = core
        .grant_capability(driver, device, OperationSet::only(Operation::Act))
        .unwrap();
    let events_before = core.events().len();

    let result = submit(&mut core, driver, capability, device, None);

    assert_eq!(result, Err(KernelError::DriverBindingNotFound));
    assert_eq!(core.driver_commands().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn submit_driver_command_requires_bound_driver_without_mutation() {
    let (mut core, owner, _, device, owner_capability, _) = prepare_bound_device::<8, 1>(
        OperationSet::empty()
            .with(Operation::Delegate)
            .with(Operation::Act),
        OperationSet::only(Operation::Act),
    );
    let events_before = core.events().len();

    let result = submit(&mut core, owner, owner_capability, device, None);

    assert_eq!(result, Err(KernelError::AgentMismatch));
    assert_eq!(core.driver_commands().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn submit_driver_command_requires_act_authority_without_mutation() {
    let (mut core, _, driver, device, _, driver_capability) = prepare_bound_device::<8, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Observe),
    );
    let events_before = core.events().len();

    let result = submit(&mut core, driver, driver_capability, device, None);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.driver_commands().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn submit_driver_command_rejects_undelivered_cause_without_mutation() {
    let (mut core, owner, driver, device, owner_capability, driver_capability) =
        prepare_bound_device::<12, 1>(
            OperationSet::empty()
                .with(Operation::Delegate)
                .with(Operation::Act),
            OperationSet::only(Operation::Act),
        );
    let cause = core
        .raise_device_event(
            owner,
            owner_capability,
            device,
            DeviceEventKind::Interrupt,
            DeviceEventPayload { code: 1, value: 2 },
        )
        .unwrap();
    let events_before = core.events().len();

    let result = submit(&mut core, driver, driver_capability, device, Some(cause));

    assert_eq!(result, Err(KernelError::DeviceEventStatusMismatch));
    assert_eq!(core.driver_commands().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn submit_driver_command_rejects_foreign_cause_without_mutation() {
    let mut core = EventKernel::<20, 1>::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let first = core.register_resource(ResourceKind::Device, None).unwrap();
    let second = core.register_resource(ResourceKind::Device, None).unwrap();
    let owner_operations = OperationSet::empty()
        .with(Operation::Delegate)
        .with(Operation::Act);
    let first_owner_capability = core
        .grant_capability(owner, first, owner_operations)
        .unwrap();
    let second_owner_capability = core
        .grant_capability(owner, second, owner_operations)
        .unwrap();
    let driver_operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act);
    let first_driver_capability = core
        .grant_capability(driver, first, driver_operations)
        .unwrap();
    let second_driver_capability = core
        .grant_capability(driver, second, driver_operations)
        .unwrap();
    core.bind_driver(owner, first_owner_capability, first, driver)
        .unwrap();
    core.bind_driver(owner, second_owner_capability, second, driver)
        .unwrap();
    let cause = core
        .raise_device_event(
            owner,
            first_owner_capability,
            first,
            DeviceEventKind::DataReady,
            DeviceEventPayload { code: 3, value: 4 },
        )
        .unwrap();
    core.deliver_device_event(driver, first_driver_capability, cause)
        .unwrap();
    let events_before = core.events().len();

    let result = submit(
        &mut core,
        driver,
        second_driver_capability,
        second,
        Some(cause),
    );

    assert_eq!(result, Err(KernelError::DriverCommandCauseMismatch));
    assert_eq!(core.driver_commands().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn submit_driver_command_missing_cause_does_not_consume_id() {
    let (mut core, _, driver, device, _, driver_capability) = prepare_bound_device::<8, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let events_before = core.events().len();

    let missing = submit(
        &mut core,
        driver,
        driver_capability,
        device,
        Some(DeviceEventId::new(99)),
    );

    assert_eq!(missing, Err(KernelError::DeviceEventNotFound));
    assert_eq!(core.driver_commands().len(), 0);
    assert_eq!(core.events().len(), events_before);
    assert_eq!(
        submit(&mut core, driver, driver_capability, device, None),
        Ok(DriverCommandId::new(1))
    );
}

#[test]
fn submit_driver_command_store_full_leaves_event_log_unchanged() {
    let (mut core, _, driver, device, _, driver_capability) = prepare_bound_device::<8, 0>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );
    let events_before = core.events().len();

    let result = submit(&mut core, driver, driver_capability, device, None);

    assert_eq!(result, Err(KernelError::DriverCommandStoreFull));
    assert_eq!(core.driver_commands().len(), 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn submit_driver_command_log_full_leaves_no_command() {
    let (mut core, _, driver, device, _, driver_capability) = prepare_bound_device::<5, 1>(
        OperationSet::only(Operation::Delegate),
        OperationSet::only(Operation::Act),
    );

    let result = submit(&mut core, driver, driver_capability, device, None);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.driver_commands().len(), 0);
    assert_eq!(core.events().len(), 5);
}
