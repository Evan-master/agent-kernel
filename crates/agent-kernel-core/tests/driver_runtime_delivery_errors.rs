mod driver_runtime_support;

use agent_kernel_core::{AgentEntryKind, AgentImageKind, DeviceEventStatus, KernelError};

use driver_runtime_support::{
    launch_entry, prepare_bound_driver, prepare_driver, raise_event, RuntimeKernel,
};

#[test]
fn delivery_store_full_is_atomic() {
    let mut core = RuntimeKernel::<32, 0>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 1);
    let events_before = core.events().len();

    let result = core.deliver_device_event(prepared.driver, prepared.driver_capability, event);

    assert_eq!(result, Err(KernelError::DriverInvocationStoreFull));
    assert!(core.driver_invocations().is_empty());
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn delivery_requires_two_event_slots_without_partial_mutation() {
    let mut core = RuntimeKernel::<11, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 2);
    let events_before = core.events().len();

    let result = core.deliver_device_event(prepared.driver, prepared.driver_capability, event);

    assert_eq!(events_before, 10);
    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.driver_invocations().is_empty());
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn delivery_rejects_unlaunched_bound_driver() {
    let mut core = RuntimeKernel::<16, 2>::new();
    let prepared = prepare_bound_driver(&mut core);
    let event = raise_event(&mut core, prepared, 3);
    let events_before = core.events().len();

    let result = core.deliver_device_event(prepared.driver, prepared.driver_capability, event);

    assert_eq!(result, Err(KernelError::AgentNotLaunched));
    assert!(core.driver_invocations().is_empty());
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn delivery_rejects_non_driver_launch_entry() {
    let mut core = RuntimeKernel::<20, 2>::new();
    let mut prepared = prepare_bound_driver(&mut core);
    launch_entry(
        &mut core,
        &mut prepared,
        AgentImageKind::Worker,
        AgentEntryKind::Worker,
    );
    let event = raise_event(&mut core, prepared, 4);
    let events_before = core.events().len();

    let result = core.deliver_device_event(prepared.driver, prepared.driver_capability, event);

    assert_eq!(result, Err(KernelError::AgentEntryKindMismatch));
    assert!(core.driver_invocations().is_empty());
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn revoked_entry_authority_blocks_delivery() {
    let mut core = RuntimeKernel::<24, 2>::new();
    let prepared = prepare_driver(&mut core);
    let entry_capability = prepared.entry_capability.unwrap();
    core.revoke_capability(entry_capability).unwrap();
    let event = raise_event(&mut core, prepared, 5);
    let events_before = core.events().len();

    let result = core.deliver_device_event(prepared.driver, prepared.driver_capability, event);

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert!(core.driver_invocations().is_empty());
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Raised);
    assert_eq!(core.events().len(), events_before);
}
