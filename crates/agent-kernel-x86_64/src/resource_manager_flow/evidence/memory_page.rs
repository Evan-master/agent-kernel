//! Terminal semantic and event evidence for the Manager runtime memory page.
//!
//! This boot-evidence child reads facade-exposed Memory Resource, Capability,
//! MemoryCell, and Event records. Physical leaf removal and frame zeroing are
//! independently retained by the completed CPU evidence checked by the parent.

use agent_kernel_core::{
    Event, EventKind, MemoryValue, Operation, OperationSet, ResourceKind, ResourceStatus,
};

use super::super::RESOURCE_MANAGER;
use crate::{boot_agent_images::BootResourceManagerImage, X86BootedKernel};

pub(super) fn state_valid(booted: &X86BootedKernel, image: BootResourceManagerImage) -> bool {
    let kernel = booted.kernel();
    let resource = kernel
        .resources()
        .iter()
        .find(|record| record.id == image.memory_resource());
    let capability = kernel.capability(image.memory_capability()).ok();
    let cell = kernel
        .memory_cells()
        .iter()
        .find(|record| record.id == image.memory_cell());
    let operations = OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);

    kernel.memory_cells().len() == 4
        && matches!(resource, Some(record)
            if record.kind == ResourceKind::Memory
                && record.parent == Some(booted.report().bootstrap_resource)
                && record.owner == Some(RESOURCE_MANAGER)
                && record.status == ResourceStatus::Retired)
        && matches!(capability, Some(record)
            if record.agent == RESOURCE_MANAGER
                && record.resource == image.memory_resource()
                && record.operations == operations
                && !record.revoked
                && record.task.is_none()
                && record.parent.is_none())
        && matches!(cell, Some(record)
            if record.resource == image.memory_resource()
                && record.creator == RESOURCE_MANAGER
                && record.last_writer == RESOURCE_MANAGER
                && record.value == descriptor(image)
                && record.revision == 1)
}

pub(super) fn events_valid(
    events: &[Event],
    booted: &X86BootedKernel,
    image: BootResourceManagerImage,
) -> bool {
    if events.len() != 5 {
        return false;
    }
    let operations = OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);
    events.iter().all(|event| event.agent == RESOURCE_MANAGER)
        && events[0].kind == EventKind::ResourceCreated
        && events[0].resource == Some(image.memory_resource())
        && events[0].capability == Some(image.memory_capability())
        && events[0].operations == operations
        && events[1].kind == EventKind::CapabilityGranted
        && events[1].resource == Some(image.memory_resource())
        && events[1].capability == Some(image.memory_capability())
        && events[1].operations == operations
        && events[2].kind == EventKind::MemoryCellCreated
        && events[2].resource == Some(image.memory_resource())
        && events[2].capability == Some(image.memory_capability())
        && events[2].memory_cell == Some(image.memory_cell())
        && events[2].operation == Some(Operation::Act)
        && events[3].kind == EventKind::MemoryCellRecalled
        && events[3].resource == Some(image.memory_resource())
        && events[3].capability == Some(image.memory_capability())
        && events[3].memory_cell == Some(image.memory_cell())
        && events[3].operation == Some(Operation::Observe)
        && events[4].kind == EventKind::ResourceRetired
        && events[4].resource == Some(image.memory_resource())
        && events[4].capability == Some(image.memory_capability())
        && booted.kernel().events().contains(&events[4])
}

fn descriptor(image: BootResourceManagerImage) -> MemoryValue {
    MemoryValue::new([
        agent_kernel_x86_64::user_memory::UserMemoryLayout::fixed().runtime_page_start(),
        agent_kernel_x86_64::user_memory::PAGE_BYTES,
        agent_kernel_x86_64::runtime_page::RUNTIME_PAGE_ACCESS_READ_WRITE,
        image.memory_generation(),
    ])
}
