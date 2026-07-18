//! Capability-checked physical runtime memory-page handlers.
//!
//! This bare-metal adapter binds one x86 retained frame to existing Memory
//! Resource and MemoryCell semantics. Reversible page-table preparation occurs
//! around public facade commits, and every reply is checked against exact core
//! records, events, physical ownership, and the running scheduler context.

use agent_kernel_core::{
    CapabilityId, EventKind, MemoryCellId, Operation, ResourceId, ResourceStatus,
};

use super::{super::state, memory_authority};
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    agent_memory::RuntimeMemoryPool,
    serial_write_line, X86BootedKernel,
};

pub(super) fn allocate(
    booted: &mut X86BootedKernel,
    memory_pool: &mut RuntimeMemoryPool,
    mut pending: PendingAgentCallCpu,
    capability: CapabilityId,
    resource: ResourceId,
) -> Option<ResumableAgentCpu> {
    let context = memory_authority::authenticated_context(&pending)?;
    if !memory_authority::valid(
        booted,
        context.agent(),
        capability,
        resource,
        Operation::Act,
    ) || !state::running(booted, context)
    {
        return None;
    }
    let pool_reservation = memory_pool.reserve(context.agent(), resource, 1)?;
    let frames = memory_pool.frame_set_for_reservation(pool_reservation)?;
    let Some((reservation, descriptor)) =
        pending.prepare_runtime_page_allocation(resource, capability, frames)
    else {
        memory_pool.cancel(pool_reservation).then_some(())?;
        return None;
    };
    let event_start = booted.kernel().events().len();
    let cell = match booted.kernel_mut().sys_create_memory_cell(
        context.agent(),
        capability,
        resource,
        descriptor,
    ) {
        Ok(cell) => cell,
        Err(_) => {
            pending
                .rollback_runtime_page_allocation(reservation, frames)
                .then_some(())?;
            memory_pool.cancel(pool_reservation).then_some(())?;
            return None;
        }
    };
    let generation = descriptor.words[3];
    if !pending.commit_runtime_page_allocation(reservation, cell)
        || !memory_pool.commit(pool_reservation, cell, generation)
    {
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
        || memory_pool
            .binding(context.agent(), resource, cell, generation)
            .is_none_or(|binding| binding.page_count() != 1)
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_ALLOCATE_MEMORY_PAGE_OK");
    pending.acknowledge_memory_page_allocated(cell, descriptor.words[0], generation)
}

pub(super) fn inspect(
    booted: &mut X86BootedKernel,
    memory_pool: &RuntimeMemoryPool,
    mut pending: PendingAgentCallCpu,
    capability: CapabilityId,
    cell: MemoryCellId,
) -> Option<ResumableAgentCpu> {
    let context = memory_authority::authenticated_context(&pending)?;
    let record = memory_authority::cell(booted, cell)?;
    if !memory_authority::valid(
        booted,
        context.agent(),
        capability,
        record.resource,
        Operation::Observe,
    ) || !state::running(booted, context)
    {
        return None;
    }
    let generation = record.value.words[3];
    let binding = memory_pool.binding(context.agent(), record.resource, cell, generation)?;
    let frames = memory_pool.frame_set_for_binding(binding)?;
    let mapped_generation =
        pending.validate_runtime_page(record.resource, cell, record.value, frames)?;
    let (value, repeated_value) = memory_pool.observe(binding)?;
    if binding.page_count() != 1 || mapped_generation != generation || repeated_value != value {
        return None;
    }
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
    pending.record_runtime_page_observation(value);
    serial_write_line("AGENT_KERNEL_AGENT_CALL_INSPECT_MEMORY_PAGE_OK");
    pending.acknowledge_memory_page_inspected(cell, value, generation)
}

pub(super) fn release(
    booted: &mut X86BootedKernel,
    memory_pool: &mut RuntimeMemoryPool,
    mut pending: PendingAgentCallCpu,
    capability: CapabilityId,
    cell: MemoryCellId,
) -> Option<ResumableAgentCpu> {
    let context = memory_authority::authenticated_context(&pending)?;
    let record = memory_authority::cell(booted, cell)?;
    if !memory_authority::valid(
        booted,
        context.agent(),
        capability,
        record.resource,
        Operation::Rollback,
    ) || !state::running(booted, context)
    {
        return None;
    }
    let generation = record.value.words[3];
    let pool_release =
        memory_pool.prepare_release(context.agent(), record.resource, cell, generation)?;
    let frames = memory_pool.frame_set_for_release(pool_release)?;
    let release =
        pending.prepare_runtime_page_release(record.resource, cell, record.value, frames)?;
    if pool_release.page_count() != 1 || release.generation() != generation {
        return None;
    }
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
        || !state::running(booted, context)
    {
        return None;
    }
    if !pending.deactivate_runtime_page(release, frames)
        || !memory_pool.release(pool_release)
        || !pending.commit_runtime_page_release(release)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RELEASE_MEMORY_PAGE_OK");
    pending.acknowledge_memory_page_released(cell, record.resource, release.generation())
}
