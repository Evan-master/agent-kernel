//! Audited Supervisor handler for terminal MemoryCell record retirement.
//!
//! The adapter rejects live architecture-owned mappings, invokes the public
//! facade, then validates exact capacity return and Event evidence.

use agent_kernel_core::{
    CapabilityId, EventKind, MemoryCellId, Operation, ResourceKind, ResourceStatus,
};

use super::super::{state, NativeExecutionReport};
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    agent_memory::RuntimeMemoryPool,
    native_agent_runtime::NativeAgentRuntime,
    serial_write_line, X86BootedKernel,
};

pub(super) fn retire(
    booted: &mut X86BootedKernel,
    runtime: &NativeAgentRuntime,
    memory_pool: &RuntimeMemoryPool,
    report: &NativeExecutionReport,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    target: MemoryCellId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    if pending.references_memory_cell(target)
        || runtime.contains_memory_cell(target)
        || memory_pool.contains_memory_cell(target)
        || report.contains_memory_cell(target)
    {
        return None;
    }

    let target_record = booted
        .kernel()
        .memory_cells()
        .iter()
        .find(|record| record.id == target)
        .copied()?;
    let resource = booted
        .kernel()
        .resources()
        .iter()
        .find(|resource| resource.id == target_record.resource)
        .copied()?;
    if resource.kind != ResourceKind::Memory || resource.status != ResourceStatus::Retired {
        return None;
    }
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let memory_cell_count = booted.kernel().memory_cells().len();

    let receipt = booted
        .kernel_mut()
        .sys_retire_memory_cell_record(context.agent(), authority, target)
        .ok()?;
    let kernel = booted.kernel();
    let event = kernel.events().get(event_start)?;
    if receipt.record() != target_record
        || receipt.memory_cell() != target
        || receipt.actor() != context.agent()
        || receipt.authority() != authority
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || kernel.memory_cells().len() + 1 != memory_cell_count
        || kernel
            .memory_cells()
            .iter()
            .any(|record| record.id == target)
        || kernel.capability(authority).is_err()
        || event.sequence != next_sequence
        || event.kind != EventKind::MemoryCellRecordRetired
        || event.agent != context.agent()
        || event.target_agent != Some(target_record.last_writer)
        || event.resource != Some(target_record.resource)
        || event.capability != Some(authority)
        || event.memory_cell != Some(target)
        || event.operation != Some(Operation::Rollback)
        || pending.references_memory_cell(target)
        || runtime.contains_memory_cell(target)
        || memory_pool.contains_memory_cell(target)
        || report.contains_memory_cell(target)
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_MEMORY_CELL_RECORD_RETIREMENT_OK");
    pending.acknowledge_memory_cell_record_retirement(receipt)
}
