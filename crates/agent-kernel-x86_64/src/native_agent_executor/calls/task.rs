//! Task and verification Agent Call handlers for the native runtime loop.

use agent_kernel_core::{EventKind, TaskId, TaskResult};
use agent_kernel_x86_64::runtime_reclamation::RuntimeReclamationLog;

use super::super::{state, NativeVerifyAuthority};
use crate::{
    agent_cpu::{CompletedAgentCpu, PendingAgentCallCpu, ResumableAgentCpu},
    X86BootedKernel,
};

pub(super) fn submit_result(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    result: TaskResult,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event = booted
        .kernel_mut()
        .sys_submit_task_result(
            context.agent(),
            context.capability(),
            context.task(),
            result,
        )
        .ok()?;
    if event.kind != EventKind::TaskResultSubmitted
        || event.agent != context.agent()
        || event.task != Some(context.task())
        || event.capability != Some(context.capability())
        || event.task_result != Some(result)
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_task_result()
}

pub(super) fn yield_running(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event = booted
        .kernel_mut()
        .sys_yield_task(context.agent(), context.task())
        .ok()?;
    if event.kind != EventKind::TaskYielded
        || event.agent != context.agent()
        || event.task != Some(context.task())
        || !state::queued(booted, context)
    {
        return None;
    }
    pending.acknowledge_yield()
}

pub(super) fn inspect_result(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    target: TaskId,
    authority: NativeVerifyAuthority,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let capability = authority.resolve(context.agent())?;
    let event = booted
        .kernel_mut()
        .sys_inspect_task_result(context.agent(), capability, target)
        .ok()?;
    let result = event.task_result?;
    if event.kind != EventKind::TaskResultInspected
        || event.agent != context.agent()
        || event.capability != Some(capability)
        || event.task != Some(target)
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_task_inspection(result)
}

pub(super) fn verify(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    target: TaskId,
    authority: NativeVerifyAuthority,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let capability = authority.resolve(context.agent())?;
    let event = booted
        .kernel_mut()
        .sys_verify_task(context.agent(), capability, target)
        .ok()?;
    if event.kind != EventKind::TaskVerified
        || event.agent != context.agent()
        || event.capability != Some(capability)
        || event.task != Some(target)
        || !state::verified(booted, target)
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_task_verification()
}

pub(super) fn complete(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    reclamation: RuntimeReclamationLog,
) -> Option<CompletedAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event = booted
        .kernel_mut()
        .sys_complete_task(context.agent(), context.capability(), context.task())
        .ok()?;
    if event.kind != EventKind::TaskCompleted
        || event.agent != context.agent()
        || event.task != Some(context.task())
        || event.capability != Some(context.capability())
        || !state::completed(booted, context)
    {
        return None;
    }
    pending.complete(reclamation)
}

pub(super) fn completion_ready(
    booted: &X86BootedKernel,
    pending: &PendingAgentCallCpu,
) -> Option<()> {
    let context = authenticated_context(pending)?;
    booted
        .kernel()
        .can_complete_task(context.agent(), context.capability(), context.task())
        .ok()
}

fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}
