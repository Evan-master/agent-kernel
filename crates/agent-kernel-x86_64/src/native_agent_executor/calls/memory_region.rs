//! Capability-checked multi-page runtime memory-region handlers.
//!
//! The executor coordinates one virtual-region ledger, the shared physical
//! frame pool, public MemoryCell calls, exact event evidence, and reply state.

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
    page_count: usize,
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
    let pool_reservation = memory_pool.reserve(context.agent(), resource, page_count)?;
    let frames = memory_pool.frame_set_for_reservation(pool_reservation)?;
    let Some((reservation, descriptor)) =
        pending.prepare_runtime_region_allocation(resource, page_count, frames)
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
                .rollback_runtime_region_allocation(reservation, frames)
                .then_some(())?;
            memory_pool.cancel(pool_reservation).then_some(())?;
            return None;
        }
    };
    let generation = descriptor.words[3];
    if !pending.commit_runtime_region_allocation(reservation, cell)
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
            .is_none_or(|binding| binding.page_count() != page_count)
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_ALLOCATE_MEMORY_REGION_OK");
    pending.acknowledge_memory_region_allocated(
        cell,
        descriptor.words[0],
        page_count as u64,
        generation,
    )
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
    let pool_binding = memory_pool.binding(context.agent(), record.resource, cell, generation)?;
    let frames = memory_pool.frame_set_for_binding(pool_binding)?;
    let region_binding =
        pending.validate_runtime_region(record.resource, cell, record.value, frames)?;
    let (first, last) = memory_pool.observe(pool_binding)?;
    if pool_binding.page_count() != region_binding.page_count()
        || pool_binding.generation() != region_binding.generation()
        || !pending.can_record_runtime_region_observation(region_binding)
    {
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
    if !pending.record_runtime_region_observation(first, last, region_binding) {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_INSPECT_MEMORY_REGION_OK");
    pending.acknowledge_memory_region_inspected(
        cell,
        first,
        last,
        region_binding.page_count() as u64,
        region_binding.generation(),
    )
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
        pending.prepare_runtime_region_release(record.resource, cell, record.value, frames)?;
    if pool_release.page_count() != release.page_count()
        || pool_release.generation() != release.generation()
    {
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
    if !pending.deactivate_runtime_region(release, frames)
        || !memory_pool.release(pool_release)
        || !pending.commit_runtime_region_release(release)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RELEASE_MEMORY_REGION_OK");
    pending.acknowledge_memory_region_released(
        cell,
        record.resource,
        release.page_count() as u64,
        release.generation(),
    )
}
