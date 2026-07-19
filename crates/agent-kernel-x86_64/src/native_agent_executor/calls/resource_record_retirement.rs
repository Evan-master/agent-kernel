//! Audited Supervisor handler for terminal Resource record retirement.
//!
//! The adapter snapshots the terminal record, invokes the public facade, then
//! validates dense capacity return, monotonic Event sequencing, and complete
//! retirement evidence before replying to the authenticated ring-3 caller.

use agent_kernel_core::{CapabilityId, EventKind, Operation, ResourceId, ResourceStatus};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn retire(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: ResourceId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let target_record = booted
        .kernel()
        .resources()
        .iter()
        .find(|record| record.id == target)
        .copied()?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let resource_count = booted.kernel().resources().len();

    let receipt = booted
        .kernel_mut()
        .sys_retire_resource_record(context.agent(), authority, target)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    if receipt.record() != target_record
        || receipt.resource() != target
        || receipt.actor() != context.agent()
        || receipt.authority() != authority
        || target_record.status != ResourceStatus::Retired
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || kernel.resources().len() + 1 != resource_count
        || kernel.resources().iter().any(|record| record.id == target)
        || kernel.capability(authority).is_err()
        || event.sequence != next_sequence
        || event.kind != EventKind::ResourceRecordRetired
        || event.agent != context.agent()
        || event.target_agent != target_record.owner
        || event.resource != Some(target)
        || event.capability != Some(authority)
        || event.operation != Some(Operation::Rollback)
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_RESOURCE_RECORD_RETIREMENT_OK");
    pending.acknowledge_resource_record_retirement(receipt)
}
