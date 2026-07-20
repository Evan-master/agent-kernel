//! Final Store and exact Event proof for hierarchical Namespace resolution.

use agent_kernel_core::{Event, NamespaceObject, Operation};

use crate::{
    boot_agent_images::BootResourceManagerImage, resource_manager_flow::RESOURCE_MANAGER,
    X86BootedKernel, X86_NAMESPACE_ENTRY_CAPACITY,
};

pub(super) fn state_valid(booted: &X86BootedKernel, image: BootResourceManagerImage) -> bool {
    let entries = booted.kernel().namespace_entries();
    let [record] = entries else {
        return false;
    };
    entries.len() == 1
        && booted.kernel().namespace_entry_capacity() == X86_NAMESPACE_ENTRY_CAPACITY
        && record.id == image.namespace_entry()
        && record.owner == RESOURCE_MANAGER
        && record.namespace == image.resource()
        && record.capability == image.capability()
        && record.key == image.namespace_key()
        && record.object == NamespaceObject::Agent(RESOURCE_MANAGER)
        && record.revision == 2
        && record.id != image.retired_namespace_entry()
        && record.key != image.retired_namespace_key()
}

pub(super) fn events_valid(
    events: &[Event],
    booted: &X86BootedKernel,
    image: BootResourceManagerImage,
) -> bool {
    let [mount_bound, terminal_bound, mount_resolved, terminal_resolved, terminal_rebound, mount_retired] =
        events
    else {
        return false;
    };
    mount_common(mount_bound, booted, image)
        && mount_bound.kind == agent_kernel_core::EventKind::NamespaceEntryBound
        && mount_bound.operation == Some(Operation::Act)
        && terminal_common(terminal_bound, image)
        && terminal_bound.kind == agent_kernel_core::EventKind::NamespaceEntryBound
        && terminal_bound.namespace_object == Some(NamespaceObject::MemoryCell(image.memory_cell()))
        && terminal_bound.operation == Some(Operation::Act)
        && mount_common(mount_resolved, booted, image)
        && mount_resolved.kind == agent_kernel_core::EventKind::NamespaceEntryResolved
        && mount_resolved.operation == Some(Operation::Observe)
        && terminal_common(terminal_resolved, image)
        && terminal_resolved.kind == agent_kernel_core::EventKind::NamespaceEntryResolved
        && terminal_resolved.namespace_object
            == Some(NamespaceObject::MemoryCell(image.memory_cell()))
        && terminal_resolved.operation == Some(Operation::Observe)
        && terminal_common(terminal_rebound, image)
        && terminal_rebound.kind == agent_kernel_core::EventKind::NamespaceEntryRebound
        && terminal_rebound.namespace_object == Some(NamespaceObject::Agent(RESOURCE_MANAGER))
        && terminal_rebound.operation == Some(Operation::Act)
        && mount_common(mount_retired, booted, image)
        && mount_retired.kind == agent_kernel_core::EventKind::NamespaceEntryRetired
        && mount_retired.operation == Some(Operation::Rollback)
        && mount_retired.target_agent == Some(RESOURCE_MANAGER)
}

fn mount_common(event: &Event, booted: &X86BootedKernel, image: BootResourceManagerImage) -> bool {
    event.agent == RESOURCE_MANAGER
        && event.resource == Some(booted.report().bootstrap_resource)
        && event.capability == Some(image.resource_authority())
        && event.namespace_entry == Some(image.retired_namespace_entry())
        && event.namespace_key == Some(image.retired_namespace_key())
        && event.namespace_object == Some(NamespaceObject::Mount(image.resource()))
}

fn terminal_common(event: &Event, image: BootResourceManagerImage) -> bool {
    event.agent == RESOURCE_MANAGER
        && event.resource == Some(image.resource())
        && event.capability == Some(image.capability())
        && event.namespace_entry == Some(image.namespace_entry())
        && event.namespace_key == Some(image.namespace_key())
}
