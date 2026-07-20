//! Audited native handler for stable dense Namespace Entry retirement.

use agent_kernel_core::{CapabilityId, EventKind, NamespaceEntryId, Operation};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel, X86_NAMESPACE_ENTRY_CAPACITY,
};

pub(super) fn retire(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    entry: NamespaceEntryId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let records = booted.kernel().namespace_entries();
    let record_count = records.len();
    if record_count > X86_NAMESPACE_ENTRY_CAPACITY {
        return None;
    }
    let mut previous = [None; X86_NAMESPACE_ENTRY_CAPACITY];
    for (slot, record) in previous.iter_mut().zip(records.iter().copied()) {
        *slot = Some(record);
    }
    let index = records.iter().position(|record| record.id == entry)?;
    let target = previous[index]?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();

    let receipt = booted
        .kernel_mut()
        .sys_retire_namespace_entry(context.agent(), authority, entry)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    if receipt.record() != target
        || receipt.actor() != context.agent()
        || receipt.authority() != authority
        || kernel.namespace_entries().len() + 1 != record_count
        || !stable_dense_retirement(kernel.namespace_entries(), previous, index)
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || event.sequence != next_sequence
        || event.kind != EventKind::NamespaceEntryRetired
        || event.agent != context.agent()
        || event.resource != Some(target.namespace)
        || event.capability != Some(authority)
        || event.namespace_entry != Some(entry)
        || event.namespace_key != Some(target.key)
        || event.namespace_object != Some(target.object)
        || event.operation != Some(Operation::Rollback)
        || event.target_agent != Some(target.owner)
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_NAMESPACE_RETIREMENT_OK");
    pending.acknowledge_namespace_retirement(receipt)
}

pub(super) fn compare_retire(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    entry: NamespaceEntryId,
    expected_revision: u64,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let records = booted.kernel().namespace_entries();
    let record_count = records.len();
    if record_count > X86_NAMESPACE_ENTRY_CAPACITY {
        return None;
    }
    let mut previous = [None; X86_NAMESPACE_ENTRY_CAPACITY];
    for (slot, record) in previous.iter_mut().zip(records.iter().copied()) {
        *slot = Some(record);
    }
    let index = records.iter().position(|record| record.id == entry)?;
    let target = previous[index]?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();

    let receipt = booted
        .kernel_mut()
        .sys_compare_and_retire_namespace_entry(
            context.agent(),
            authority,
            entry,
            expected_revision,
        )
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    if receipt.record() != target
        || target.revision != expected_revision
        || receipt.actor() != context.agent()
        || receipt.authority() != authority
        || kernel.namespace_entries().len() + 1 != record_count
        || !stable_dense_retirement(kernel.namespace_entries(), previous, index)
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || event.sequence != next_sequence
        || event.kind != EventKind::NamespaceEntryRetired
        || event.agent != context.agent()
        || event.resource != Some(target.namespace)
        || event.capability != Some(authority)
        || event.namespace_entry != Some(entry)
        || event.namespace_key != Some(target.key)
        || event.namespace_object != Some(target.object)
        || event.operation != Some(Operation::Rollback)
        || event.target_agent != Some(target.owner)
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_NAMESPACE_COMPARE_RETIRE_OK");
    pending.acknowledge_namespace_compare_retirement(receipt)
}

fn stable_dense_retirement(
    retained: &[agent_kernel_core::NamespaceEntryRecord],
    previous: [Option<agent_kernel_core::NamespaceEntryRecord>; X86_NAMESPACE_ENTRY_CAPACITY],
    retired_index: usize,
) -> bool {
    retained.iter().enumerate().all(|(index, record)| {
        let previous_index = if index < retired_index {
            index
        } else {
            index + 1
        };
        previous.get(previous_index).copied().flatten() == Some(*record)
    })
}
