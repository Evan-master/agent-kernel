//! Capability-checked managed Agent lifecycle handlers.
//!
//! This bare-metal adapter authenticates physical call ownership, invokes only
//! public facade methods, and verifies exact Agent records, execution context,
//! and event consequences before returning a canonical reply to ring 3.

use agent_kernel_core::{
    AgentExecutionState, AgentId, AgentStatus, CapabilityId, EventKind, Operation, ResourceId,
};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn register(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    resource: ResourceId,
    target: AgentId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let event = booted
        .kernel_mut()
        .sys_register_managed_agent(context.agent(), authority, resource, target)
        .ok()?;
    if event != *booted.kernel().events().get(event_start)?
        || !managed_state_valid(
            booted,
            context,
            event_start,
            target,
            resource,
            authority,
            AgentStatus::Active,
            EventKind::AgentRegistered,
        )
        || booted.kernel().agents().last()?.manager != Some(context.agent())
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_REGISTER_MANAGED_AGENT_OK");
    pending.acknowledge_agent_management(target, resource, AgentStatus::Active)
}

pub(super) fn suspend(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: AgentId,
) -> Option<ResumableAgentCpu> {
    transition(
        booted,
        pending,
        authority,
        target,
        AgentStatus::Suspended,
        EventKind::AgentSuspended,
        ManagedTransition::Suspend,
        "AGENT_KERNEL_AGENT_CALL_SUSPEND_MANAGED_AGENT_OK",
    )
}

pub(super) fn resume(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: AgentId,
) -> Option<ResumableAgentCpu> {
    transition(
        booted,
        pending,
        authority,
        target,
        AgentStatus::Active,
        EventKind::AgentResumed,
        ManagedTransition::Resume,
        "AGENT_KERNEL_AGENT_CALL_RESUME_MANAGED_AGENT_OK",
    )
}

pub(super) fn retire(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: AgentId,
) -> Option<ResumableAgentCpu> {
    transition(
        booted,
        pending,
        authority,
        target,
        AgentStatus::Retired,
        EventKind::AgentRetired,
        ManagedTransition::Retire,
        "AGENT_KERNEL_AGENT_CALL_RETIRE_MANAGED_AGENT_OK",
    )
}

#[derive(Copy, Clone)]
enum ManagedTransition {
    Suspend,
    Resume,
    Retire,
}

#[allow(clippy::too_many_arguments)]
fn transition(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: AgentId,
    status: AgentStatus,
    kind: EventKind,
    transition: ManagedTransition,
    marker: &str,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let resource = booted
        .kernel()
        .agents()
        .iter()
        .find(|record| record.id == target)?
        .management_resource?;
    let event_start = booted.kernel().events().len();
    let event = match transition {
        ManagedTransition::Suspend => {
            booted
                .kernel_mut()
                .sys_suspend_managed_agent(context.agent(), authority, target)
        }
        ManagedTransition::Resume => {
            booted
                .kernel_mut()
                .sys_resume_managed_agent(context.agent(), authority, target)
        }
        ManagedTransition::Retire => {
            booted
                .kernel_mut()
                .sys_retire_managed_agent(context.agent(), authority, target)
        }
    }
    .ok()?;
    if event != *booted.kernel().events().get(event_start)?
        || !managed_state_valid(
            booted,
            context,
            event_start,
            target,
            resource,
            authority,
            status,
            kind,
        )
    {
        return None;
    }
    serial_write_line(marker);
    pending.acknowledge_agent_management(target, resource, status)
}

#[allow(clippy::too_many_arguments)]
fn managed_state_valid(
    booted: &X86BootedKernel,
    context: agent_kernel_x86_64::agent_call::AgentCallContext,
    event_start: usize,
    target: AgentId,
    resource: ResourceId,
    authority: CapabilityId,
    status: AgentStatus,
    kind: EventKind,
) -> bool {
    let kernel = booted.kernel();
    let record = kernel.agents().iter().find(|record| record.id == target);
    let execution = kernel
        .execution_contexts()
        .iter()
        .find(|record| record.agent == target);
    let event = kernel.events().get(event_start);
    kernel.events().len() == event_start + 1
        && matches!(record, Some(record)
            if record.status == status
                && record.management_resource == Some(resource)
                && record.manager.is_some())
        && matches!(execution, Some(execution)
            if execution.state == AgentExecutionState::Idle
                && execution.task.is_none()
                && execution.driver_invocation.is_none())
        && !kernel
            .agent_entries()
            .iter()
            .any(|entry| entry.agent == target)
        && matches!(event, Some(event)
            if event.kind == kind
                && event.agent == context.agent()
                && event.target_agent == Some(target)
                && event.resource == Some(resource)
                && event.capability == Some(authority)
                && event.operation == Some(Operation::Delegate))
        && state::running(booted, context)
}

fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}
