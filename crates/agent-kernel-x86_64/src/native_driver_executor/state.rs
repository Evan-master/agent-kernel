//! Trusted Driver execution-state predicates.
//!
//! This child binds an authenticated call context to one active Driver entry,
//! Capability, Core Invocation, and execution context without exposing device
//! coordinates to ring 3.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, DriverInvocationRecord, DriverInvocationStatus, Operation,
};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::{agent_cpu::PendingAgentCallCpu, X86BootedKernel};

pub(super) fn authenticated_context(
    booted: &X86BootedKernel,
    pending: &PendingAgentCallCpu,
) -> Option<AgentCallContext> {
    pending.authenticated_request()?;
    let context = pending.context();
    running_invocation(booted, context)?;
    Some(context)
}

pub(super) fn running_invocation(
    booted: &X86BootedKernel,
    context: AgentCallContext,
) -> Option<DriverInvocationRecord> {
    let invocation_id = context.driver_invocation()?;
    let kernel = booted.kernel();
    let invocation = kernel
        .driver_invocations()
        .iter()
        .find(|record| record.id == invocation_id)
        .copied()?;
    let entry = kernel.agent_entry(context.agent()).ok()?;
    let capability = kernel.capability(context.capability()).ok()?;
    let execution = kernel
        .execution_contexts()
        .iter()
        .find(|record| record.agent == context.agent())?;
    (invocation.status == DriverInvocationStatus::Running
        && invocation.driver == context.agent()
        && entry.kind == AgentEntryKind::Driver
        && entry.image == context.image()
        && entry.resource == invocation.resource
        && entry.capability == context.capability()
        && entry.task.is_none()
        && capability.agent == context.agent()
        && capability.resource == invocation.resource
        && capability.operations.allows(Operation::Observe)
        && capability.operations.allows(Operation::Act)
        && !capability.revoked
        && execution.state == AgentExecutionState::Running
        && execution.task.is_none()
        && execution.driver_invocation == Some(invocation_id))
    .then_some(invocation)
}
