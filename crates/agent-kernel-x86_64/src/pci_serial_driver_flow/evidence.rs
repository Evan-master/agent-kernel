//! Terminal Core-record evidence for the native PCI serial command.
//!
//! The checker reads public fixed-capacity stores after physical execution and
//! requires one coherent Binding, Event, Command, Invocation, and idle context.

use agent_kernel_core::{
    AgentExecutionState, AgentId, DeviceEventId, DeviceEventKind, DeviceEventPayload,
    DeviceEventStatus, DriverBindingId, DriverCommandId, DriverCommandKind, DriverCommandPayload,
    DriverCommandResult, DriverCommandStatus, DriverInvocationId, DriverInvocationStatus,
    FaultKind, ResourceId,
};

use crate::X86BootedKernel;

pub(super) struct TerminalEvidence {
    pub(super) driver: AgentId,
    pub(super) resource: ResourceId,
    pub(super) binding: DriverBindingId,
    pub(super) event: DeviceEventId,
    pub(super) event_kind: DeviceEventKind,
    pub(super) event_payload: DeviceEventPayload,
    pub(super) command: DriverCommandId,
    pub(super) command_kind: DriverCommandKind,
    pub(super) command_payload: DriverCommandPayload,
    pub(super) invocation: DriverInvocationId,
    pub(super) result: DriverCommandResult,
    pub(super) run_ticks: u64,
    pub(super) restart_generation: u8,
    pub(super) fault: Option<(FaultKind, u64)>,
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
            && record.kind == expected.event_kind
            && record.payload == expected.event_payload
    }) && command.is_some_and(|record| {
        record.binding == expected.binding
            && record.resource == expected.resource
            && record.cause == Some(expected.event)
            && record.invocation == Some(expected.invocation)
            && record.status == DriverCommandStatus::Completed
            && record.kind == expected.command_kind
            && record.payload == expected.command_payload
            && record.result == Some(expected.result)
    }) && invocation.is_some_and(|record| {
        record.binding == expected.binding
            && record.resource == expected.resource
            && record.event == expected.event
            && record.status == DriverInvocationStatus::Completed
            && record.run_ticks == expected.run_ticks
            && record.restart_generation == expected.restart_generation
            && record.fault_kind.zip(record.fault_detail) == expected.fault
    }) && context.is_some_and(|record| record.state == AgentExecutionState::Idle)
}
