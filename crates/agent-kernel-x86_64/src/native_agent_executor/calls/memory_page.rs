//! Capability-checked physical runtime memory-page handlers.
//!
//! This bare-metal adapter binds one x86 retained frame to existing Memory
//! Resource and MemoryCell semantics. Reversible page-table preparation occurs
//! around public facade commits, and every reply is checked against exact core
//! records, events, physical ownership, and the running scheduler context.

use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, MemoryCellId, MemoryCellRecord, Operation, ResourceId,
    ResourceKind, ResourceStatus,
};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn allocate(
    booted: &mut X86BootedKernel,
    mut pending: PendingAgentCallCpu,
    capability: CapabilityId,
    resource: ResourceId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    if !authority_valid(
        booted,
        context.agent(),
        capability,
        resource,
        Operation::Act,
    ) || !state::running(booted, context)
    {
        return None;
    }
    let (reservation, descriptor) = pending.prepare_runtime_page_allocation(resource)?;
    let event_start = booted.kernel().events().len();
    let cell = match booted.kernel_mut().sys_create_memory_cell(
        context.agent(),
        capability,
        resource,
        descriptor,
    ) {
        Ok(cell) => cell,
        Err(_) => {
            pending.rollback_runtime_page_allocation(reservation);
            return None;
        }
    };
    if !pending.commit_runtime_page_allocation(reservation, cell) {
        return None;
    }
    let kernel = booted.kernel();
    let record = kernel
        .memory_cells()
        .iter()
        .find(|record| record.id == cell)?;
    let event = kernel.events().get(event_start)?;
    if kernel.events().len() != event_start + 1
        || record.resource != resource
        || record.creator != context.agent()
        || record.last_writer != context.agent()
        || record.value != descriptor
        || record.revision != 1
        || event.kind != EventKind::MemoryCellCreated
        || event.agent != context.agent()
        || event.resource != Some(resource)
        || event.capability != Some(capability)
        || event.memory_cell != Some(cell)
        || event.operation != Some(Operation::Act)
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_ALLOCATE_MEMORY_PAGE_OK");
    pending.acknowledge_memory_page_allocated(cell, descriptor.words[0], descriptor.words[3])
}

pub(super) fn inspect(
    booted: &mut X86BootedKernel,
    mut pending: PendingAgentCallCpu,
    capability: CapabilityId,
    cell: MemoryCellId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let record = memory_cell(booted, cell)?;
    if !authority_valid(
        booted,
        context.agent(),
        capability,
        record.resource,
        Operation::Observe,
    ) || !state::running(booted, context)
    {
        return None;
    }
    let (value, generation) = pending.inspect_runtime_page(record.resource, cell, record.value)?;
    let event_start = booted.kernel().events().len();
    let recalled = booted
        .kernel_mut()
        .sys_recall_memory_cell(context.agent(), capability, cell)
        .ok()?;
    let event = booted.kernel().events().get(event_start)?;
    if recalled != record.value
        || booted.kernel().events().len() != event_start + 1
        || event.kind != EventKind::MemoryCellRecalled
        || event.agent != context.agent()
        || event.resource != Some(record.resource)
        || event.capability != Some(capability)
        || event.memory_cell != Some(cell)
        || event.operation != Some(Operation::Observe)
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_INSPECT_MEMORY_PAGE_OK");
    pending.acknowledge_memory_page_inspected(cell, value, generation)
}

pub(super) fn release(
    booted: &mut X86BootedKernel,
    mut pending: PendingAgentCallCpu,
    capability: CapabilityId,
    cell: MemoryCellId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let record = memory_cell(booted, cell)?;
    if !authority_valid(
        booted,
        context.agent(),
        capability,
        record.resource,
        Operation::Rollback,
    ) || !state::running(booted, context)
    {
        return None;
    }
    let release = pending.prepare_runtime_page_release(record.resource, cell, record.value)?;
    let event_start = booted.kernel().events().len();
    let event = booted
        .kernel_mut()
        .sys_retire_resource(context.agent(), capability, record.resource)
        .ok()?;
    let resource = booted
        .kernel()
        .resources()
        .iter()
        .find(|resource| resource.id == record.resource)?;
    if booted.kernel().events().len() != event_start + 1
        || event != booted.kernel().events()[event_start]
        || event.kind != EventKind::ResourceRetired
        || event.agent != context.agent()
        || event.resource != Some(record.resource)
        || event.capability != Some(capability)
        || resource.status != ResourceStatus::Retired
        || !pending.release_runtime_page(release)
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RELEASE_MEMORY_PAGE_OK");
    pending.acknowledge_memory_page_released(cell, record.resource, release.generation())
}

fn memory_cell(booted: &X86BootedKernel, cell: MemoryCellId) -> Option<MemoryCellRecord> {
    booted
        .kernel()
        .memory_cells()
        .iter()
        .find(|record| record.id == cell)
        .copied()
}

fn authority_valid(
    booted: &X86BootedKernel,
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    operation: Operation,
) -> bool {
    let kernel = booted.kernel();
    matches!(kernel.resources().iter().find(|record| record.id == resource), Some(record)
        if record.kind == ResourceKind::Memory && record.status == ResourceStatus::Active)
        && matches!(kernel.capability(capability), Ok(record)
            if record.agent == agent
                && record.resource == resource
                && record.operations.allows(operation)
                && !record.revoked
                && record.task.is_none())
}

fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}
