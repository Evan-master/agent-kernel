//! Authenticated Driver Call handlers for one native ring-3 session.
//!
//! This child translates semantic requests into public Core transitions and
//! one immutable HAL dispatch. It never derives physical endpoint coordinates
//! from ring-3 input.

use agent_kernel_core::{
    AgentExecutionState, DeviceEventId, DeviceEventStatus, DriverCommandKind, DriverCommandPayload,
    DriverCommandStatus, EventKind,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{state, CommandEvidence, DriverRunProgress};
use crate::{
    agent_cpu::{AgentRunOutcome, CompletedAgentCpu, PendingAgentCallCpu, ResumableAgentCpu},
    port_driver_flow::record_command_outcome,
    X86BootedKernel,
};

pub(super) fn continue_run<B: DriverBackend>(
    booted: &mut X86BootedKernel,
    backend: &mut B,
    command: &mut Option<CommandEvidence>,
    mut outcome: AgentRunOutcome,
) -> Option<DriverRunProgress> {
    loop {
        match outcome {
            AgentRunOutcome::Call(pending) => {
                let request = pending.request();
                let resumable = match request {
                    AgentCallRequest::DescribeContext { .. } => pending.acknowledge_describe()?,
                    AgentCallRequest::InspectDriverInvocation { .. } => {
                        inspect_invocation(booted, pending)?
                    }
                    AgentCallRequest::AcknowledgeDeviceEvent { event, .. } => {
                        acknowledge_event(booted, pending, event)?
                    }
                    AgentCallRequest::SubmitDriverCommand {
                        event,
                        kind,
                        payload,
                        ..
                    } => {
                        if command.is_some() {
                            return None;
                        }
                        let (cpu, evidence) =
                            submit_command(booted, pending, event, kind, payload, backend)?;
                        *command = Some(evidence);
                        cpu
                    }
                    AgentCallRequest::CompleteDriverInvocation { .. } => {
                        let completed = complete_invocation(booted, pending, *command.as_ref()?)?;
                        return Some(DriverRunProgress::Completed(completed));
                    }
                    _ => return None,
                };
                outcome = resumable.resume_until_boundary()?;
            }
            AgentRunOutcome::Preempted(cpu) => {
                return Some(DriverRunProgress::Preempted(cpu));
            }
            AgentRunOutcome::Fault(cpu) => {
                return Some(DriverRunProgress::Faulted(cpu));
            }
        }
    }
}

fn inspect_invocation(
    booted: &X86BootedKernel,
    pending: PendingAgentCallCpu,
) -> Option<ResumableAgentCpu> {
    let context = state::authenticated_context(booted, &pending)?;
    let invocation = state::running_invocation(booted, context)?;
    let event = booted
        .kernel()
        .device_events()
        .iter()
        .find(|record| record.id == invocation.event)?;
    if event.binding != invocation.binding
        || event.resource != invocation.resource
        || event.status != DeviceEventStatus::Delivered
    {
        return None;
    }
    pending.acknowledge_driver_invocation(
        event.id,
        event.resource,
        event.binding,
        event.kind,
        event.payload,
    )
}

fn acknowledge_event(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    event: DeviceEventId,
) -> Option<ResumableAgentCpu> {
    let context = state::authenticated_context(booted, &pending)?;
    let invocation = state::running_invocation(booted, context)?;
    if event != invocation.event {
        return None;
    }
    let transition = booted
        .kernel_mut()
        .sys_acknowledge_device_event(context.agent(), context.capability(), event)
        .ok()?;
    if transition.kind != EventKind::DeviceEventAcknowledged
        || transition.agent != context.agent()
        || transition.device_event != Some(event)
        || transition.driver_invocation != Some(invocation.id)
        || transition.capability != Some(context.capability())
        || state::running_invocation(booted, context).is_none()
    {
        return None;
    }
    pending.acknowledge_device_event(event)
}

fn submit_command<B: DriverBackend>(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    event: DeviceEventId,
    kind: DriverCommandKind,
    payload: DriverCommandPayload,
    backend: &mut B,
) -> Option<(ResumableAgentCpu, CommandEvidence)> {
    let context = state::authenticated_context(booted, &pending)?;
    let invocation = state::running_invocation(booted, context)?;
    let event_record = booted
        .kernel()
        .device_events()
        .iter()
        .find(|record| record.id == event)?;
    if event != invocation.event
        || event_record.status != DeviceEventStatus::Acknowledged
        || event_record.resource != invocation.resource
        || event_record.binding != invocation.binding
    {
        return None;
    }
    let command = booted
        .kernel_mut()
        .sys_submit_driver_command(
            context.agent(),
            context.capability(),
            invocation.resource,
            Some(event),
            kind,
            payload,
        )
        .ok()?;
    let request = booted
        .kernel_mut()
        .sys_dispatch_driver_command(context.agent(), context.capability(), command)
        .ok()?;
    if request.command != command
        || request.binding != invocation.binding
        || request.resource != invocation.resource
        || request.driver != context.agent()
        || request.cause != Some(event)
        || request.invocation != Some(invocation.id)
        || request.kind != kind
        || request.payload != payload
    {
        return None;
    }

    let outcome = backend.execute(request);
    let result = outcome.result();
    if !record_command_outcome(
        booted,
        context.agent(),
        context.capability(),
        command,
        outcome,
    ) || !matches!(outcome, DriverCommandOutcome::Completed(_))
    {
        return None;
    }
    let resumable = pending.acknowledge_driver_command(command, result)?;
    Some((
        resumable,
        CommandEvidence {
            id: command,
            result,
        },
    ))
}

fn complete_invocation(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    command: CommandEvidence,
) -> Option<CompletedAgentCpu> {
    let context = state::authenticated_context(booted, &pending)?;
    let invocation = state::running_invocation(booted, context)?;
    let command_record = booted
        .kernel()
        .driver_commands()
        .iter()
        .find(|record| record.id == command.id)?;
    if command_record.driver != context.agent()
        || command_record.resource != invocation.resource
        || command_record.binding != invocation.binding
        || command_record.invocation != Some(invocation.id)
        || command_record.cause != Some(invocation.event)
        || command_record.status != DriverCommandStatus::Completed
        || command_record.result != Some(command.result)
    {
        return None;
    }
    let transition = booted
        .kernel_mut()
        .sys_complete_driver_invocation(context.agent(), context.capability(), invocation.id)
        .ok()?;
    let execution = booted
        .kernel()
        .execution_contexts()
        .iter()
        .find(|record| record.agent == context.agent())?;
    if transition.kind != EventKind::DriverInvocationCompleted
        || transition.driver_invocation != Some(invocation.id)
        || execution.state != AgentExecutionState::Idle
        || execution.driver_invocation.is_some()
    {
        return None;
    }
    pending.complete_driver()
}
