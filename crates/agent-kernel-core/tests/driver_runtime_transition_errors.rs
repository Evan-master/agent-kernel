mod driver_runtime_support;

use agent_kernel_core::{
    AgentExecutionState, AgentId, DeviceEventStatus, DriverCommandKind, DriverCommandPayload,
    DriverInvocationStatus, KernelError,
};

use driver_runtime_support::{prepare_driver, raise_event, RuntimeKernel};

#[test]
fn queued_invocation_cannot_acknowledge_event() {
    let mut core = RuntimeKernel::<24, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 1);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    let events_before = core.events().len();

    let result = core.acknowledge_device_event(prepared.driver, prepared.driver_capability, event);

    assert_eq!(result, Err(KernelError::DriverInvocationNotRunnable));
    assert_eq!(core.device_events()[0].status, DeviceEventStatus::Delivered);
    assert_eq!(core.driver_invocations()[0].id, invocation);
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Queued
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn queued_invocation_cannot_submit_causal_command() {
    let mut core = RuntimeKernel::<24, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 2);
    core.deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    let events_before = core.events().len();

    let result = core.submit_driver_command(
        prepared.driver,
        prepared.driver_capability,
        prepared.device,
        Some(event),
        DriverCommandKind::Write,
        DriverCommandPayload {
            opcode: 1,
            value: 2,
        },
    );

    assert_eq!(result, Err(KernelError::DriverInvocationNotRunnable));
    assert!(core.driver_commands().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn invocation_completion_requires_acknowledged_event() {
    let mut core = RuntimeKernel::<24, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 3);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 2)
        .unwrap();
    let events_before = core.events().len();

    let result =
        core.complete_driver_invocation(prepared.driver, prepared.driver_capability, invocation);

    assert_eq!(result, Err(KernelError::DeviceEventStatusMismatch));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Running
    );
    assert_eq!(
        core.execution_context(prepared.driver).unwrap().state,
        AgentExecutionState::Running
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn completion_log_full_leaves_invocation_running() {
    let mut core = RuntimeKernel::<14, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 4);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 2)
        .unwrap();
    core.acknowledge_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();

    let result =
        core.complete_driver_invocation(prepared.driver, prepared.driver_capability, invocation);

    assert_eq!(core.events().len(), 14);
    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Running
    );
    let context = core.execution_context(prepared.driver).unwrap();
    assert_eq!(context.state, AgentExecutionState::Running);
    assert_eq!(context.driver_invocation, Some(invocation));
}

#[test]
fn revoked_entry_authority_blocks_running_invocation_tick() {
    let mut core = RuntimeKernel::<24, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 5);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 2)
        .unwrap();
    core.revoke_capability(prepared.entry_capability.unwrap())
        .unwrap();
    let events_before = core.events().len();

    let result = core.tick_driver_invocation(prepared.driver, invocation);

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Running
    );
    assert_eq!(core.driver_invocations()[0].run_ticks, 0);
    assert_eq!(core.driver_invocations()[0].quantum_remaining, 2);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn dispatch_rejects_zero_quantum_and_empty_queue() {
    let mut core = RuntimeKernel::<16, 2>::new();
    let prepared = prepare_driver(&mut core);
    let events_before = core.events().len();

    assert_eq!(
        core.dispatch_next_driver_invocation(prepared.driver, 0),
        Err(KernelError::DriverInvocationQuantumInvalid)
    );
    assert_eq!(
        core.dispatch_next_driver_invocation(prepared.driver, 1),
        Err(KernelError::DriverInvocationQueueEmpty)
    );
    assert_eq!(
        core.execution_context(prepared.driver).unwrap().state,
        AgentExecutionState::Idle
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn retired_resource_blocks_queued_invocation_dispatch() {
    let mut core = RuntimeKernel::<24, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 6);
    core.deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.retire_resource(prepared.owner, prepared.owner_capability, prepared.device)
        .unwrap();
    let events_before = core.events().len();

    let result = core.dispatch_next_driver_invocation(prepared.driver, 2);

    assert_eq!(result, Err(KernelError::ResourceRetired));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Queued
    );
    assert_eq!(
        core.execution_context(prepared.driver).unwrap().state,
        AgentExecutionState::Idle
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn another_agent_cannot_complete_running_invocation() {
    let mut core = RuntimeKernel::<24, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 7);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 2)
        .unwrap();
    let other = AgentId::new(3);
    core.register_agent(other).unwrap();
    let events_before = core.events().len();

    let result = core.complete_driver_invocation(other, prepared.driver_capability, invocation);

    assert_eq!(result, Err(KernelError::AgentMismatch));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Running
    );
    assert_eq!(
        core.execution_context(prepared.driver)
            .unwrap()
            .driver_invocation,
        Some(invocation)
    );
    assert_eq!(core.events().len(), events_before);
}
