mod driver_runtime_support;

use agent_kernel_core::{
    AgentExecutionState, AgentId, DeviceEventStatus, DriverCommandKind, DriverCommandPayload,
    DriverInvocationStatus, EventKind, FaultKind, KernelError, Operation, OperationSet,
};
use driver_runtime_support::{prepare_driver, raise_event, RuntimeKernel};

type TestKernel = RuntimeKernel<40, 3>;

#[test]
fn owner_recovers_one_faulted_driver_generation_without_losing_invocation_identity() {
    let mut core = TestKernel::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 7);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 2)
        .unwrap();

    let fault = core
        .fault_driver_invocation(prepared.driver, invocation, FaultKind::ExecutionTrap, 6)
        .expect("the running Driver Invocation should fault");

    assert_eq!(fault.kind, EventKind::DriverInvocationFaulted);
    assert_eq!(fault.fault_kind, Some(FaultKind::ExecutionTrap));
    assert_eq!(fault.fault_detail, Some(6));
    assert_eq!(fault.driver_invocation, Some(invocation));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Faulted
    );
    assert_eq!(
        core.driver_invocations()[0].fault_kind,
        Some(FaultKind::ExecutionTrap)
    );
    assert_eq!(core.driver_invocations()[0].fault_detail, Some(6));
    assert_eq!(core.driver_invocations()[0].restart_generation, 0);
    let context = core.execution_context(prepared.driver).unwrap();
    assert_eq!(context.state, AgentExecutionState::Faulted);
    assert_eq!(context.task, None);
    assert_eq!(context.driver_invocation, Some(invocation));

    let recovery = core
        .recover_driver_invocation(
            prepared.owner,
            prepared.owner_capability,
            prepared.driver,
            invocation,
        )
        .expect("Rollback authority should recover generation zero");

    assert_eq!(recovery.kind, EventKind::DriverInvocationRecovered);
    assert_eq!(recovery.agent, prepared.owner);
    assert_eq!(recovery.target_agent, Some(prepared.driver));
    assert_eq!(recovery.fault_kind, Some(FaultKind::ExecutionTrap));
    assert_eq!(recovery.fault_detail, Some(6));
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Queued
    );
    assert_eq!(core.driver_invocations()[0].restart_generation, 1);
    assert_eq!(
        core.execution_context(prepared.driver).unwrap().state,
        AgentExecutionState::Idle
    );
    assert_eq!(
        core.dispatch_next_driver_invocation(prepared.driver, 2)
            .unwrap(),
        invocation
    );
}

#[test]
fn driver_cannot_self_recover_without_rollback_authority() {
    let mut core = TestKernel::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 8);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 1)
        .unwrap();
    core.fault_driver_invocation(prepared.driver, invocation, FaultKind::ExecutionTrap, 6)
        .unwrap();
    let before = core.events().len();

    assert_eq!(
        core.recover_driver_invocation(
            prepared.driver,
            prepared.driver_capability,
            prepared.driver,
            invocation,
        ),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.events().len(), before);
    assert_eq!(
        core.driver_invocations()[0].status,
        DriverInvocationStatus::Faulted
    );
}

#[test]
fn recovery_rejects_acknowledged_events_and_command_evidence() {
    let mut acknowledged_core = TestKernel::new();
    let acknowledged = prepare_driver(&mut acknowledged_core);
    let event = raise_event(&mut acknowledged_core, acknowledged, 9);
    let invocation = acknowledged_core
        .deliver_device_event(acknowledged.driver, acknowledged.driver_capability, event)
        .unwrap();
    acknowledged_core
        .dispatch_next_driver_invocation(acknowledged.driver, 2)
        .unwrap();
    acknowledged_core
        .acknowledge_device_event(acknowledged.driver, acknowledged.driver_capability, event)
        .unwrap();
    acknowledged_core
        .fault_driver_invocation(acknowledged.driver, invocation, FaultKind::ExecutionTrap, 6)
        .unwrap();

    assert_eq!(
        acknowledged_core.recover_driver_invocation(
            acknowledged.owner,
            acknowledged.owner_capability,
            acknowledged.driver,
            invocation,
        ),
        Err(KernelError::DriverInvocationRecoveryUnsafe)
    );
    assert_eq!(
        acknowledged_core.device_events()[0].status,
        DeviceEventStatus::Acknowledged
    );

    let mut command_core = TestKernel::new();
    let command = prepare_driver(&mut command_core);
    let event = raise_event(&mut command_core, command, 10);
    let invocation = command_core
        .deliver_device_event(command.driver, command.driver_capability, event)
        .unwrap();
    command_core
        .dispatch_next_driver_invocation(command.driver, 2)
        .unwrap();
    command_core
        .submit_driver_command(
            command.driver,
            command.driver_capability,
            command.device,
            Some(event),
            DriverCommandKind::Configure,
            DriverCommandPayload {
                opcode: 1,
                value: 0,
            },
        )
        .unwrap();
    command_core
        .fault_driver_invocation(command.driver, invocation, FaultKind::ExecutionTrap, 6)
        .unwrap();

    assert_eq!(
        command_core.recover_driver_invocation(
            command.owner,
            command.owner_capability,
            command.driver,
            invocation,
        ),
        Err(KernelError::DriverInvocationRecoveryUnsafe)
    );
}

#[test]
fn second_driver_fault_is_terminal_for_the_bounded_policy() {
    let mut core = TestKernel::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 11);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 1)
        .unwrap();
    core.fault_driver_invocation(prepared.driver, invocation, FaultKind::ExecutionTrap, 6)
        .unwrap();
    core.recover_driver_invocation(
        prepared.owner,
        prepared.owner_capability,
        prepared.driver,
        invocation,
    )
    .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 1)
        .unwrap();
    core.fault_driver_invocation(prepared.driver, invocation, FaultKind::ExecutionTrap, 6)
        .unwrap();
    let before = core.events().len();

    assert_eq!(
        core.recover_driver_invocation(
            prepared.owner,
            prepared.owner_capability,
            prepared.driver,
            invocation,
        ),
        Err(KernelError::DriverInvocationRestartLimitReached)
    );
    assert_eq!(core.events().len(), before);
    assert_eq!(core.driver_invocations()[0].restart_generation, 1);
}

#[test]
fn recovery_requires_the_exact_driver_identity() {
    let mut core = TestKernel::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 12);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 1)
        .unwrap();
    core.fault_driver_invocation(prepared.driver, invocation, FaultKind::ExecutionTrap, 6)
        .unwrap();
    let third = AgentId::new(3);
    core.register_agent(third).unwrap();
    let third_capability = core
        .grant_capability(
            third,
            prepared.device,
            OperationSet::empty().with(Operation::Rollback),
        )
        .unwrap();

    assert_eq!(
        core.recover_driver_invocation(third, third_capability, third, invocation),
        Err(KernelError::AgentMismatch)
    );
}

#[test]
fn fault_is_fail_before_write_when_the_event_log_is_full() {
    let mut core = RuntimeKernel::<13, 1>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 13);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 1)
        .unwrap();
    let before_record = core.driver_invocations()[0];
    let before_context = core.execution_context(prepared.driver).unwrap();

    assert_eq!(
        core.fault_driver_invocation(prepared.driver, invocation, FaultKind::ExecutionTrap, 6),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.driver_invocations()[0], before_record);
    assert_eq!(
        core.execution_context(prepared.driver).unwrap(),
        before_context
    );
    assert_eq!(core.events().len(), 13);
}

#[test]
fn recovery_is_fail_before_write_when_the_event_log_is_full() {
    let mut core = RuntimeKernel::<14, 1>::new();
    let prepared = prepare_driver(&mut core);
    let event = raise_event(&mut core, prepared, 14);
    let invocation = core
        .deliver_device_event(prepared.driver, prepared.driver_capability, event)
        .unwrap();
    core.dispatch_next_driver_invocation(prepared.driver, 1)
        .unwrap();
    core.fault_driver_invocation(prepared.driver, invocation, FaultKind::ExecutionTrap, 6)
        .unwrap();
    let before_record = core.driver_invocations()[0];
    let before_context = core.execution_context(prepared.driver).unwrap();

    assert_eq!(
        core.recover_driver_invocation(
            prepared.owner,
            prepared.owner_capability,
            prepared.driver,
            invocation,
        ),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.driver_invocations()[0], before_record);
    assert_eq!(
        core.execution_context(prepared.driver).unwrap(),
        before_context
    );
    assert_eq!(core.events().len(), 14);
}
