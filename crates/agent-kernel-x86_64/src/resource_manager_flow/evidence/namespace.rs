//! Final Store and exact Event proof for Namespace capacity reuse.

use agent_kernel_core::{Event, NamespaceObject, Operation};

use crate::{
    boot_agent_images::BootResourceManagerImage, resource_manager_flow::RESOURCE_MANAGER,
    X86BootedKernel, X86_NAMESPACE_ENTRY_CAPACITY,
};

pub(super) fn state_valid(booted: &X86BootedKernel, image: BootResourceManagerImage) -> bool {
    let entries = booted.kernel().namespace_entries();
    entries.len() == X86_NAMESPACE_ENTRY_CAPACITY
        && booted.kernel().namespace_entry_capacity() == X86_NAMESPACE_ENTRY_CAPACITY
        && entries.iter().all(|record| {
            record.id == image.namespace_entry()
                && record.owner == RESOURCE_MANAGER
                && record.namespace == booted.report().bootstrap_resource
                && record.capability == image.resource_authority()
                && record.key == image.namespace_key()
                && record.object == NamespaceObject::Resource(booted.report().bootstrap_resource)
                && record.revision == 1
        })
        && entries
            .iter()
            .all(|record| record.id != image.retired_namespace_entry())
        && entries
            .iter()
            .all(|record| record.key != image.retired_namespace_key())
}

pub(super) fn events_valid(
    events: &[Event],
    booted: &X86BootedKernel,
    image: BootResourceManagerImage,
) -> bool {
    let [bound, resolved, rebound, retired, fresh] = events else {
        return false;
    };
    let namespace = booted.report().bootstrap_resource;
    let authority = image.resource_authority();
    common(bound, image, namespace, authority)
        && bound.kind == agent_kernel_core::EventKind::NamespaceEntryBound
        && bound.namespace_object == Some(NamespaceObject::MemoryCell(image.memory_cell()))
        && bound.operation == Some(Operation::Act)
        && common(resolved, image, namespace, authority)
        && resolved.kind == agent_kernel_core::EventKind::NamespaceEntryResolved
        && resolved.namespace_object == Some(NamespaceObject::MemoryCell(image.memory_cell()))
        && resolved.operation == Some(Operation::Observe)
        && common(rebound, image, namespace, authority)
        && rebound.kind == agent_kernel_core::EventKind::NamespaceEntryRebound
        && rebound.namespace_object == Some(NamespaceObject::Agent(RESOURCE_MANAGER))
        && rebound.operation == Some(Operation::Act)
        && common(retired, image, namespace, authority)
        && retired.kind == agent_kernel_core::EventKind::NamespaceEntryRetired
        && retired.namespace_object == Some(NamespaceObject::Agent(RESOURCE_MANAGER))
        && retired.operation == Some(Operation::Rollback)
        && retired.target_agent == Some(RESOURCE_MANAGER)
        && fresh.agent == RESOURCE_MANAGER
        && fresh.kind == agent_kernel_core::EventKind::NamespaceEntryBound
        && fresh.resource == Some(namespace)
        && fresh.capability == Some(authority)
        && fresh.namespace_entry == Some(image.namespace_entry())
        && fresh.namespace_key == Some(image.namespace_key())
        && fresh.namespace_object == Some(NamespaceObject::Resource(namespace))
        && fresh.operation == Some(Operation::Act)
}

fn common(
    event: &Event,
    image: BootResourceManagerImage,
    namespace: agent_kernel_core::ResourceId,
    authority: agent_kernel_core::CapabilityId,
) -> bool {
    event.agent == RESOURCE_MANAGER
        && event.resource == Some(namespace)
        && event.capability == Some(authority)
        && event.namespace_entry == Some(image.retired_namespace_entry())
        && event.namespace_key == Some(image.retired_namespace_key())
}
