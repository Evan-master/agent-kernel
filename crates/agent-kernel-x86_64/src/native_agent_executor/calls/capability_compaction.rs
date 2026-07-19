//! Audited Supervisor handler for native Capability Store compaction.

use agent_kernel_core::{CapabilityId, EventKind, KernelError, Operation};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn compact(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: CapabilityId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let task_len = booted.kernel().tasks().len();
    let admission_len = booted.kernel().runtime_admissions().len();
    let capability_capacity = booted.kernel().capability_capacity();
    let capability_count = booted.kernel().capability_count();
    let target_record = booted.kernel().capability(target).ok()?;

    let receipt = booted
        .kernel_mut()
        .sys_compact_capability(context.agent(), authority, target)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    if receipt.capability() != target
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || kernel.capability_capacity() != capability_capacity
        || kernel.capability_count() + 1 != capability_count
        || kernel.capability(target) != Err(KernelError::CapabilityNotFound)
        || kernel.capability(authority).is_err()
        || kernel.tasks().len() != task_len
        || kernel.runtime_admissions().len() != admission_len
        || event.sequence != next_sequence
        || event.kind != EventKind::CapabilityCompacted
        || event.agent != context.agent()
        || event.capability != Some(target)
        || event.source_capability != Some(authority)
        || event.operation != Some(Operation::Rollback)
        || event.resource != Some(target_record.resource)
        || event.operations != target_record.operations
        || event.task != target_record.task
        || event.target_agent != Some(target_record.agent)
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK");
    pending.acknowledge_capability_compaction(receipt)
}
