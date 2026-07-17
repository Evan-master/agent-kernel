//! Capability-checked resource lifecycle handlers for native Agent Calls.
//!
//! This bare-metal adapter authenticates physical call ownership, invokes only
//! public `agent-kernel` facade operations, and binds successful replies to the
//! exact records and event suffix produced by the deterministic core.

use agent_kernel_core::{
    CapabilityId, EventKind, OperationSet, ResourceId, ResourceKind, ResourceStatus,
};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    X86BootedKernel,
};

pub(super) fn create(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    parent: ResourceId,
    kind: ResourceKind,
    operations: OperationSet,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let outcome = booted
        .kernel_mut()
        .sys_create_resource(context.agent(), kind, Some((parent, authority)), operations)
        .ok()?;
    let kernel = booted.kernel();
    let events = kernel.events().get(event_start..)?;
    let resource = kernel
        .resources()
        .iter()
        .find(|record| record.id == outcome.resource)?;
    let capability = kernel.capability(outcome.capability).ok()?;
    if events.len() != 2
        || events[0].kind != EventKind::ResourceCreated
        || events[0].agent != context.agent()
        || events[0].resource != Some(outcome.resource)
        || events[0].capability != Some(outcome.capability)
        || events[0].operations != operations
        || events[1].kind != EventKind::CapabilityGranted
        || events[1].agent != context.agent()
        || events[1].resource != Some(outcome.resource)
        || events[1].capability != Some(outcome.capability)
        || events[1].operations != operations
        || resource.kind != kind
        || resource.parent != Some(parent)
        || resource.owner != Some(context.agent())
        || resource.status != ResourceStatus::Active
        || capability.agent != context.agent()
        || capability.resource != outcome.resource
        || capability.operations != operations
        || capability.revoked
        || capability.task.is_some()
        || capability.parent.is_some()
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_resource_created(outcome)
}

pub(super) fn retire(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    resource: ResourceId,
    capability: CapabilityId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let event = booted
        .kernel_mut()
        .sys_retire_resource(context.agent(), capability, resource)
        .ok()?;
    let record = booted
        .kernel()
        .resources()
        .iter()
        .find(|record| record.id == resource)?;
    if booted.kernel().events().len() != event_start + 1
        || event != booted.kernel().events()[event_start]
        || event.kind != EventKind::ResourceRetired
        || event.agent != context.agent()
        || event.resource != Some(resource)
        || event.capability != Some(capability)
        || record.status != ResourceStatus::Retired
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_resource_retired(resource, capability)
}

fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}
