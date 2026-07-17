//! Capability-checked Intent and Task lifecycle handlers for native calls.
//!
//! This bare-metal adapter authenticates physical call ownership, invokes the
//! public facade, and validates exact records and event suffixes before ring 3
//! receives any kernel-issued handle.

use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, IntentId, IntentKind, IntentStatus, Operation, OperationSet,
    ResourceId, TaskId, TaskStatus, VerificationRequirement,
};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn declare(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    resource: ResourceId,
    kind: IntentKind,
    verification: VerificationRequirement,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let intent = booted
        .kernel_mut()
        .sys_declare_intent(context.agent(), authority, resource, kind, verification)
        .ok()?;
    let kernel = booted.kernel();
    let record = kernel.intents().iter().find(|record| record.id == intent)?;
    let event = kernel.events().get(event_start)?;
    if kernel.events().len() != event_start + 1
        || event.kind != EventKind::IntentDeclared
        || event.agent != context.agent()
        || event.capability != Some(authority)
        || event.resource != Some(resource)
        || event.intent != Some(intent)
        || event.intent_kind != Some(kind)
        || event.operation != Some(kind.required_operation())
        || event.verification != verification
        || record.owner != context.agent()
        || record.resource != resource
        || record.kind != kind
        || record.status != IntentStatus::Declared
        || record.verification != verification
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_DECLARE_INTENT_OK");
    pending.acknowledge_intent_declared(intent)
}

pub(super) fn create(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    intent: IntentId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let task = booted
        .kernel_mut()
        .sys_create_task(context.agent(), authority, intent)
        .ok()?;
    let kernel = booted.kernel();
    let intent_record = kernel.intents().iter().find(|record| record.id == intent)?;
    let task_record = kernel.tasks().iter().find(|record| record.id == task)?;
    let events = kernel.events().get(event_start..)?;
    if events.len() != 2
        || events[0].kind != EventKind::TaskCreated
        || events[0].agent != context.agent()
        || events[0].capability != Some(authority)
        || events[0].resource != Some(intent_record.resource)
        || events[0].intent != Some(intent)
        || events[0].task != Some(task)
        || events[1].kind != EventKind::IntentBound
        || events[1].agent != context.agent()
        || events[1].resource != Some(intent_record.resource)
        || events[1].intent != Some(intent)
        || events[1].task != Some(task)
        || intent_record.owner != context.agent()
        || intent_record.status != IntentStatus::Bound
        || task_record.intent != intent
        || task_record.owner != context.agent()
        || task_record.resource != intent_record.resource
        || task_record.status != TaskStatus::Created
        || task_record.assignee.is_some()
        || task_record.delegated_capability.is_some()
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_CREATE_TASK_OK");
    pending.acknowledge_task_created(task)
}

pub(super) fn delegate(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    task: TaskId,
    target: AgentId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let event = booted
        .kernel_mut()
        .sys_delegate_task(context.agent(), authority, task, target)
        .ok()?;
    let capability = event.capability?;
    let kernel = booted.kernel();
    let task_record = kernel.tasks().iter().find(|record| record.id == task)?;
    let capability_record = kernel.capability(capability).ok()?;
    let events = kernel.events().get(event_start..)?;
    if events.len() != 2
        || events[0].kind != EventKind::CapabilityDerived
        || events[0].agent != context.agent()
        || events[0].resource != Some(task_record.resource)
        || events[0].capability != Some(capability)
        || events[0].source_capability != Some(authority)
        || events[0].operations != OperationSet::only(Operation::Act)
        || events[0].intent != Some(task_record.intent)
        || events[0].task != Some(task)
        || events[0].target_agent != Some(target)
        || event != events[1]
        || event.kind != EventKind::DelegationRequested
        || event.agent != context.agent()
        || event.task != Some(task)
        || event.target_agent != Some(target)
        || task_record.status != TaskStatus::Delegated
        || task_record.assignee != Some(target)
        || task_record.delegated_capability != Some(capability)
        || capability_record.agent != target
        || capability_record.resource != task_record.resource
        || capability_record.operations != OperationSet::only(Operation::Act)
        || capability_record.revoked
        || capability_record.task != Some(task)
        || capability_record.parent != Some(authority)
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_DELEGATE_TASK_OK");
    pending.acknowledge_task_delegated(task, capability, target)
}

fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}
