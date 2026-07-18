//! Terminal semantic, event, and observation evidence for Manager regions.
//!
//! This child validates three interleaved region lifecycles, exact descriptors,
//! ordered ring-3 proof observations, and facade-visible authority records.

use agent_kernel_core::{
    CapabilityId, Event, EventKind, MemoryCellId, MemoryValue, Operation, OperationSet, ResourceId,
    ResourceKind, ResourceStatus,
};

use super::super::RESOURCE_MANAGER;
use crate::{
    agent_cpu::CompletedAgentCpu, boot_agent_images::BootResourceManagerImage, X86BootedKernel,
};

#[derive(Copy, Clone)]
struct RegionSpec {
    resource: ResourceId,
    capability: CapabilityId,
    cell: MemoryCellId,
    start_slot: usize,
    page_count: usize,
    generation: u64,
    first: u64,
    last: u64,
}

pub(super) fn state_valid(booted: &X86BootedKernel, image: BootResourceManagerImage) -> bool {
    let kernel = booted.kernel();
    kernel.memory_cells().len() == 4
        && region_specs(image).into_iter().all(|spec| {
            let resource = kernel
                .resources()
                .iter()
                .find(|record| record.id == spec.resource);
            let capability = kernel.capability(spec.capability).ok();
            let cell = kernel
                .memory_cells()
                .iter()
                .find(|record| record.id == spec.cell);
            matches!(resource, Some(record)
                if record.kind == ResourceKind::Memory
                    && record.parent == Some(booted.report().bootstrap_resource)
                    && record.owner == Some(RESOURCE_MANAGER)
                    && record.status == ResourceStatus::Retired)
                && matches!(capability, Some(record)
                    if record.agent == RESOURCE_MANAGER
                        && record.resource == spec.resource
                        && record.operations == memory_operations()
                        && !record.revoked
                        && record.task.is_none()
                        && record.parent.is_none())
                && matches!(cell, Some(record)
                    if record.resource == spec.resource
                        && record.creator == RESOURCE_MANAGER
                        && record.last_writer == RESOURCE_MANAGER
                        && record.value == descriptor(spec)
                        && record.revision == 1)
        })
}

pub(super) fn observations_valid(
    completed: &CompletedAgentCpu,
    image: BootResourceManagerImage,
) -> bool {
    let observations = completed.runtime_region_observations();
    observations.len() == 3
        && region_specs(image)
            .into_iter()
            .enumerate()
            .all(|(index, spec)| {
                observations.get(index).is_some_and(|observation| {
                    observation.cell() == spec.cell
                        && observation.start_slot() == spec.start_slot
                        && observation.page_count() == spec.page_count
                        && observation.generation() == spec.generation
                        && observation.first() == spec.first
                        && observation.last() == spec.last
                })
            })
}

pub(super) fn events_valid(
    events: &[Event],
    _booted: &X86BootedKernel,
    image: BootResourceManagerImage,
) -> bool {
    if events.len() != 15 || !events.iter().all(|event| event.agent == RESOURCE_MANAGER) {
        return false;
    }
    let event_indices = [[0, 1, 2, 3, 7], [4, 5, 6, 8, 13], [9, 10, 11, 12, 14]];
    region_specs(image)
        .into_iter()
        .zip(event_indices)
        .all(|(spec, indices)| region_events_valid(events, indices, spec))
}

fn region_events_valid(events: &[Event], indices: [usize; 5], spec: RegionSpec) -> bool {
    let [created, granted, cell_created, recalled, retired] = indices.map(|index| &events[index]);
    created.kind == EventKind::ResourceCreated
        && created.resource == Some(spec.resource)
        && created.capability == Some(spec.capability)
        && created.operations == memory_operations()
        && granted.kind == EventKind::CapabilityGranted
        && granted.resource == Some(spec.resource)
        && granted.capability == Some(spec.capability)
        && granted.operations == memory_operations()
        && cell_created.kind == EventKind::MemoryCellCreated
        && cell_created.resource == Some(spec.resource)
        && cell_created.capability == Some(spec.capability)
        && cell_created.memory_cell == Some(spec.cell)
        && cell_created.operation == Some(Operation::Act)
        && recalled.kind == EventKind::MemoryCellRecalled
        && recalled.resource == Some(spec.resource)
        && recalled.capability == Some(spec.capability)
        && recalled.memory_cell == Some(spec.cell)
        && recalled.operation == Some(Operation::Observe)
        && retired.kind == EventKind::ResourceRetired
        && retired.resource == Some(spec.resource)
        && retired.capability == Some(spec.capability)
}

fn descriptor(spec: RegionSpec) -> MemoryValue {
    let layout = agent_kernel_x86_64::user_memory::UserMemoryLayout::fixed();
    MemoryValue::new([
        layout
            .runtime_region_page_start(spec.start_slot)
            .unwrap_or(0),
        agent_kernel_x86_64::user_memory::PAGE_BYTES * spec.page_count as u64,
        agent_kernel_x86_64::runtime_region::RUNTIME_MEMORY_ACCESS_READ_WRITE,
        spec.generation,
    ])
}

fn region_specs(image: BootResourceManagerImage) -> [RegionSpec; 3] {
    [
        RegionSpec {
            resource: image.memory_region_resource(),
            capability: image.memory_region_capability(),
            cell: image.memory_region_cell(),
            start_slot: 0,
            page_count: image.memory_region_page_count() as usize,
            generation: image.memory_region_generation(),
            first: image.memory_region_first_proof(),
            last: image.memory_region_last_proof(),
        },
        RegionSpec {
            resource: image.memory_region_b_resource(),
            capability: image.memory_region_b_capability(),
            cell: image.memory_region_b_cell(),
            start_slot: 3,
            page_count: image.memory_region_b_page_count() as usize,
            generation: image.memory_region_b_generation(),
            first: image.memory_region_b_first_proof(),
            last: image.memory_region_b_last_proof(),
        },
        RegionSpec {
            resource: image.memory_region_c_resource(),
            capability: image.memory_region_c_capability(),
            cell: image.memory_region_c_cell(),
            start_slot: 0,
            page_count: image.memory_region_c_page_count() as usize,
            generation: image.memory_region_c_generation(),
            first: image.memory_region_c_first_proof(),
            last: image.memory_region_c_last_proof(),
        },
    ]
}

fn memory_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
}
