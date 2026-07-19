//! Audited Supervisor handler for retired-Resource Capability revocation.
//!
//! The adapter invokes the public facade and proves that one active target was
//! marked revoked without changing Store occupancy. It binds the reply to the
//! exact target, Resource, ancestor authority, and ordered Rollback Event.

use agent_kernel_core::{CapabilityId, EventKind, Operation, ResourceStatus};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn revoke(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: CapabilityId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let target_record = booted.kernel().capability(target).ok()?;
    let resource = booted
        .kernel()
        .resources()
        .iter()
        .find(|record| record.id == target_record.resource)
        .copied()?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let capability_capacity = booted.kernel().capability_capacity();
    let capability_count = booted.kernel().capability_count();

    let recorded = booted
        .kernel_mut()
        .sys_revoke_capability_for_cleanup(context.agent(), authority, target)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    let mut expected_target = target_record;
    expected_target.revoked = true;
    if target_record.revoked
        || resource.status != ResourceStatus::Retired
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || kernel.capability_capacity() != capability_capacity
        || kernel.capability_count() != capability_count
        || kernel.capability(target).ok()? != expected_target
        || kernel.capability(authority).is_err()
        || *event != recorded
        || event.sequence != next_sequence
        || event.kind != EventKind::CapabilityRevoked
        || event.agent != context.agent()
        || event.resource != Some(target_record.resource)
        || event.capability != Some(target)
        || event.source_capability != Some(authority)
        || event.operation != Some(Operation::Rollback)
        || event.operations != target_record.operations
        || event.task != target_record.task
        || event.target_agent != Some(target_record.agent)
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_CAPABILITY_CLEANUP_REVOCATION_OK");
    pending.acknowledge_capability_cleanup_revocation(target, target_record.resource)
}
