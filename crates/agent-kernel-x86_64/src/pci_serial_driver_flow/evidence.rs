//! Terminal Core-record evidence for the native PCI serial command.
//!
//! The checker reads public fixed-capacity stores after physical execution and
//! requires one coherent Binding, Event, Command, Invocation, and idle context.

use agent_kernel_core::{
    AgentExecutionState, AgentId, DeviceEventId, DeviceEventStatus, DriverBindingId,
    DriverCommandId, DriverCommandResult, DriverCommandStatus, DriverInvocationId,
    DriverInvocationStatus, ResourceId,
};

use crate::X86BootedKernel;

pub(super) struct TerminalEvidence {
    pub(super) driver: AgentId,
    pub(super) resource: ResourceId,
    pub(super) binding: DriverBindingId,
    pub(super) event: DeviceEventId,
    pub(super) command: DriverCommandId,
    pub(super) invocation: DriverInvocationId,
    pub(super) result: DriverCommandResult,
}

pub(super) fn terminal_matches(booted: &X86BootedKernel, expected: TerminalEvidence) -> bool {
    let kernel = booted.kernel();
    let binding = kernel
        .driver_bindings()
        .iter()
        .find(|record| record.id == expected.binding);
    let event = kernel
        .device_events()
        .iter()
        .find(|record| record.id == expected.event);
    let command = kernel
        .driver_commands()
        .iter()
        .find(|record| record.id == expected.command);
    let invocation = kernel
        .driver_invocations()
        .iter()
        .find(|record| record.id == expected.invocation);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == expected.driver);

    binding.is_some_and(|record| {
        record.resource == expected.resource && record.driver == expected.driver
    }) && event.is_some_and(|record| {
        record.binding == expected.binding
            && record.resource == expected.resource
            && record.status == DeviceEventStatus::Acknowledged
    }) && command.is_some_and(|record| {
        record.binding == expected.binding
            && record.resource == expected.resource
            && record.cause == Some(expected.event)
            && record.invocation == Some(expected.invocation)
            && record.status == DriverCommandStatus::Completed
            && record.result == Some(expected.result)
    }) && invocation.is_some_and(|record| {
        record.binding == expected.binding
            && record.resource == expected.resource
            && record.event == expected.event
            && record.status == DriverInvocationStatus::Completed
            && record.run_ticks == 1
    }) && context.is_some_and(|record| record.state == AgentExecutionState::Idle)
}
