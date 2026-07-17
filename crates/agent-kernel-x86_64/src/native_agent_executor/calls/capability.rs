//! Capability-checked authority lifecycle handlers for native Agent Calls.
//!
//! This bare-metal adapter invokes only public facade operations. It validates
//! the exact capability records and events before returning kernel-issued
//! handles to the authenticated ring-3 caller.

use agent_kernel_core::{AgentId, CapabilityId, EventKind, Operation, OperationSet};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    X86BootedKernel,
};

pub(super) fn derive(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    source: CapabilityId,
    target: AgentId,
    operations: OperationSet,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let derived = booted
        .kernel_mut()
        .sys_derive_capability(context.agent(), source, target, operations)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    let source_record = kernel.capability(source).ok()?;
    let derived_record = kernel.capability(derived).ok()?;
    if kernel.events().len() != event_start + 1
        || event.kind != EventKind::CapabilityDerived
        || event.agent != context.agent()
        || event.resource != Some(source_record.resource)
        || event.capability != Some(derived)
        || event.source_capability != Some(source)
        || event.operations != operations
        || event.task.is_some()
        || event.target_agent != Some(target)
        || source_record.agent != context.agent()
        || source_record.task.is_some()
        || source_record.revoked
        || !source_record.operations.allows(Operation::Delegate)
        || derived_record.agent != target
        || derived_record.resource != source_record.resource
        || derived_record.operations != operations
        || derived_record.revoked
        || derived_record.task.is_some()
        || derived_record.parent != Some(source)
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_capability_derived(derived)
}

pub(super) fn revoke(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    source: CapabilityId,
    target: CapabilityId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let event = booted
        .kernel_mut()
        .sys_revoke_derived_capability(context.agent(), source, target)
        .ok()?;
    let kernel = booted.kernel();
    let source_record = kernel.capability(source).ok()?;
    let target_record = kernel.capability(target).ok()?;
    if kernel.events().len() != event_start + 1
        || event != kernel.events()[event_start]
        || event.kind != EventKind::CapabilityRevoked
        || event.agent != context.agent()
        || event.resource != Some(source_record.resource)
        || event.capability != Some(target)
        || event.source_capability != Some(source)
        || event.operations != target_record.operations
        || event.task != target_record.task
        || event.target_agent != Some(target_record.agent)
        || source_record.agent != context.agent()
        || source_record.task.is_some()
        || source_record.revoked
        || !source_record.operations.allows(Operation::Delegate)
        || target_record.resource != source_record.resource
        || target_record.parent != Some(source)
        || !target_record.revoked
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_capability_revoked(source, target)
}

fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}
