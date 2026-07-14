mod driver_runtime_support;

use agent_kernel_core::{AgentExecutionState, DriverInvocationStatus, EventKind, KernelError};

use driver_runtime_support::{prepare_driver, raise_event, RuntimeKernel};

#[test]
fn driver_invocations_dispatch_in_fifo_order() {
    let mut core = RuntimeKernel::<40, 3>::new();
    let prepared = prepare_driver(&mut core);
    let first_event = raise_event(&mut core, prepared, 1);
    let second_event = raise_event(&mut core, prepared, 2);
    let first = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, first_event)
        .unwrap();
    let second = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, second_event)
        .unwrap();

    assert_eq!(
        core.dispatch_next_driver_invocation(prepared.driver, 2),
        Ok(first)
    );
    let events_before = core.events().len();
    assert_eq!(
        core.dispatch_next_driver_invocation(prepared.driver, 2),
        Err(KernelError::ExecutionContextBusy)
    );
    assert_eq!(
        core.driver_invocations()[1].status,
        DriverInvocationStatus::Queued
    );
    assert_eq!(core.events().len(), events_before);

    core.acknowledge_device_event(prepared.driver, prepared.driver_capability, first_event)
        .unwrap();
    core.complete_driver_invocation(prepared.driver, prepared.driver_capability, first)
        .unwrap();
    assert_eq!(
        core.dispatch_next_driver_invocation(prepared.driver, 3),
        Ok(second)
    );
}

#[test]
fn quantum_expiry_requeues_and_preserves_tick_progress() {
    let mut core = RuntimeKernel::<32, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 3);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 2)
        .unwrap();

    let tick = core
        .tick_driver_invocation(prepared.driver, invocation)
        .unwrap();
    assert_eq!(tick.kind, EventKind::DriverInvocationTicked);
    assert_eq!(tick.driver_invocation_ticks, Some(1));
    assert_eq!(tick.driver_invocation_quantum, Some(1));
    assert_eq!(core.driver_invocations()[0].run_ticks, 1);
    assert_eq!(core.driver_invocations()[0].quantum_remaining, 1);

    let expiry = core
        .tick_driver_invocation(prepared.driver, invocation)
        .unwrap();
    assert_eq!(expiry.kind, EventKind::DriverInvocationQuantumExpired);
    assert_eq!(expiry.driver_invocation_ticks, Some(2));
    assert_eq!(expiry.driver_invocation_quantum, Some(0));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Queued
    );
    assert_eq!(core.driver_invocations()[0].run_ticks, 2);
    assert_eq!(
        core.execution_context(prepared.driver).unwrap().state,
        AgentExecutionState::Idle
    );

    assert_eq!(
        core.dispatch_next_driver_invocation(prepared.driver, 3),
        Ok(invocation)
    );
    let context = core.execution_context(prepared.driver).unwrap();
    assert_eq!(context.driver_invocation, Some(invocation));
    assert_eq!(context.run_ticks, 2);
    assert_eq!(context.quantum_remaining, 3);
}

#[test]
fn tick_log_full_leaves_progress_unchanged() {
    let mut core = RuntimeKernel::<13, 2>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 4);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 2)
        .unwrap();

    let result = core.tick_driver_invocation(prepared.driver, invocation);

    assert_eq!(core.events().len(), 13);
    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Running
    );
    assert_eq!(core.driver_invocations()[0].run_ticks, 0);
    assert_eq!(core.driver_invocations()[0].quantum_remaining, 2);
    let context = core.execution_context(prepared.driver).unwrap();
    assert_eq!(context.run_ticks, 0);
    assert_eq!(context.quantum_remaining, 2);
}
