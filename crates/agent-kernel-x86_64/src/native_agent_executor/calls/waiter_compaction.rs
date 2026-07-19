//! Audited Supervisor handler for native Waiter Store compaction.
//!
//! This x86 adapter snapshots the bounded pre-state, invokes the public facade,
//! and validates stable removal plus complete ordered Event evidence before it
//! returns to the authenticated ring-3 caller.

use agent_kernel_core::{CapabilityId, EventKind, Operation, WaiterId, WaiterRecord};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel, X86_WAITER_CAPACITY,
};

pub(super) fn compact(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    through: WaiterId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let waiter_len = booted.kernel().waiters().len();
    if waiter_len > X86_WAITER_CAPACITY {
        return None;
    }
    let mut previous: [Option<WaiterRecord>; X86_WAITER_CAPACITY] = [None; X86_WAITER_CAPACITY];
    for (index, record) in booted.kernel().waiters().iter().copied().enumerate() {
        previous[index] = Some(record);
    }
    let through_index = booted
        .kernel()
        .waiters()
        .iter()
        .position(|record| record.id == through)?;
    let expected_count = through_index + 1;
    let event_start = booted.kernel().events().len();
    let queue_len = booted.kernel().run_queue().len();

    let receipt = booted
        .kernel_mut()
        .sys_compact_waiter_prefix(context.agent(), authority, through)
        .ok()?;
    let kernel = booted.kernel();
    let events = kernel.events().get(event_start..)?;
    if receipt.first() != previous[0]?.id
        || receipt.through() != through
        || receipt.count() != expected_count
        || kernel.waiters().len() + receipt.count() != waiter_len
        || kernel.run_queue().len() != queue_len
        || kernel.waiters().iter().enumerate().any(|(index, record)| {
            previous.get(index + receipt.count()).copied().flatten() != Some(*record)
        })
        || events.len() != receipt.count()
        || events.iter().enumerate().any(|(index, event)| {
            let Some(record) = previous[index] else {
                return true;
            };
            event.sequence != (event_start + index + 1) as u64
                || event.kind != EventKind::WaiterCompacted
                || event.agent != context.agent()
                || event.target_agent != Some(record.agent)
                || event.resource != Some(record.resource)
                || event.capability != Some(authority)
                || event.operation != Some(Operation::Rollback)
                || event.task != Some(record.task)
                || event.waiter != Some(record.id)
                || event.waiter_kind != Some(record.kind)
                || event.signal != Some(record.signal)
                || record.active
        })
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_WAITER_COMPACTION_OK");
    pending.acknowledge_waiter_compaction(receipt)
}
