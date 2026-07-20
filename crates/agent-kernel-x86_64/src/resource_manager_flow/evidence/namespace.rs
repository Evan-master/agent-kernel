//! Final Store and exact Event proof for four-hop Namespace resolution and mutation.

use agent_kernel_core::{
    Event, EventKind, NamespaceEntryId, NamespaceKey, NamespaceObject, Operation, ResourceId,
    ResourceKind, ResourceStatus,
};

use crate::{
    boot_agent_images::BootResourceManagerImage, resource_manager_flow::RESOURCE_MANAGER,
    X86BootedKernel, X86_NAMESPACE_ENTRY_CAPACITY,
};

pub(super) fn state_valid(booted: &X86BootedKernel, image: BootResourceManagerImage) -> bool {
    let kernel = booted.kernel();
    let entries = kernel.namespace_entries();
    let Some(root) = find_entry(entries, image.root_namespace_entry()) else {
        return false;
    };
    let Some(child) = find_entry(entries, image.namespace_entry()) else {
        return false;
    };
    let Some(third) = find_entry(entries, image.path_entry_a()) else {
        return false;
    };
    let Some(terminal) = find_entry(entries, image.path_entry_b()) else {
        return false;
    };

    entries.len() == 4
        && kernel.namespace_entry_capacity() == X86_NAMESPACE_ENTRY_CAPACITY
        && entry_matches(
            root,
            booted.report().bootstrap_resource,
            image.resource_authority(),
            image.root_namespace_key(),
            NamespaceObject::Mount(image.resource()),
            1,
        )
        && entry_matches(
            child,
            image.resource(),
            image.capability(),
            image.namespace_key(),
            NamespaceObject::Mount(image.path_workspace_a()),
            2,
        )
        && entry_matches(
            third,
            image.path_workspace_a(),
            image.path_capability_a(),
            image.path_key_a(),
            NamespaceObject::Mount(image.path_workspace_b()),
            1,
        )
        && entry_matches(
            terminal,
            image.path_workspace_b(),
            image.path_capability_b(),
            image.path_key_b(),
            NamespaceObject::Resource(image.resource()),
            2,
        )
        && workspace_valid(booted, image.path_workspace_a())
        && workspace_valid(booted, image.path_workspace_b())
}

pub(super) fn events_valid(
    events: &[Event],
    booted: &X86BootedKernel,
    image: BootResourceManagerImage,
) -> bool {
    let [root_bound, child_bound, root_short, child_short, child_rebound, third_bound, terminal_bound, root_long, child_long, third_long, terminal_long, root_mutation, child_mutation, third_mutation, terminal_rebound] =
        events
    else {
        return false;
    };
    let root_resource = booted.report().bootstrap_resource;

    event_matches(
        root_bound,
        EventKind::NamespaceEntryBound,
        root_resource,
        image.resource_authority(),
        image.root_namespace_entry(),
        image.root_namespace_key(),
        NamespaceObject::Mount(image.resource()),
        Operation::Act,
    ) && event_matches(
        child_bound,
        EventKind::NamespaceEntryBound,
        image.resource(),
        image.capability(),
        image.namespace_entry(),
        image.namespace_key(),
        NamespaceObject::MemoryCell(image.memory_cell()),
        Operation::Act,
    ) && event_matches(
        root_short,
        EventKind::NamespaceEntryResolved,
        root_resource,
        image.resource_authority(),
        image.root_namespace_entry(),
        image.root_namespace_key(),
        NamespaceObject::Mount(image.resource()),
        Operation::Observe,
    ) && event_matches(
        child_short,
        EventKind::NamespaceEntryResolved,
        image.resource(),
        image.capability(),
        image.namespace_entry(),
        image.namespace_key(),
        NamespaceObject::MemoryCell(image.memory_cell()),
        Operation::Observe,
    ) && event_matches(
        child_rebound,
        EventKind::NamespaceEntryRebound,
        image.resource(),
        image.capability(),
        image.namespace_entry(),
        image.namespace_key(),
        NamespaceObject::Mount(image.path_workspace_a()),
        Operation::Act,
    ) && event_matches(
        third_bound,
        EventKind::NamespaceEntryBound,
        image.path_workspace_a(),
        image.path_capability_a(),
        image.path_entry_a(),
        image.path_key_a(),
        NamespaceObject::Mount(image.path_workspace_b()),
        Operation::Act,
    ) && event_matches(
        terminal_bound,
        EventKind::NamespaceEntryBound,
        image.path_workspace_b(),
        image.path_capability_b(),
        image.path_entry_b(),
        image.path_key_b(),
        NamespaceObject::Agent(RESOURCE_MANAGER),
        Operation::Act,
    ) && event_matches(
        root_long,
        EventKind::NamespaceEntryResolved,
        root_resource,
        image.resource_authority(),
        image.root_namespace_entry(),
        image.root_namespace_key(),
        NamespaceObject::Mount(image.resource()),
        Operation::Observe,
    ) && event_matches(
        child_long,
        EventKind::NamespaceEntryResolved,
        image.resource(),
        image.capability(),
        image.namespace_entry(),
        image.namespace_key(),
        NamespaceObject::Mount(image.path_workspace_a()),
        Operation::Observe,
    ) && event_matches(
        third_long,
        EventKind::NamespaceEntryResolved,
        image.path_workspace_a(),
        image.path_capability_a(),
        image.path_entry_a(),
        image.path_key_a(),
        NamespaceObject::Mount(image.path_workspace_b()),
        Operation::Observe,
    ) && event_matches(
        terminal_long,
        EventKind::NamespaceEntryResolved,
        image.path_workspace_b(),
        image.path_capability_b(),
        image.path_entry_b(),
        image.path_key_b(),
        NamespaceObject::Agent(RESOURCE_MANAGER),
        Operation::Observe,
    ) && event_matches(
        root_mutation,
        EventKind::NamespaceEntryResolved,
        root_resource,
        image.resource_authority(),
        image.root_namespace_entry(),
        image.root_namespace_key(),
        NamespaceObject::Mount(image.resource()),
        Operation::Observe,
    ) && event_matches(
        child_mutation,
        EventKind::NamespaceEntryResolved,
        image.resource(),
        image.capability(),
        image.namespace_entry(),
        image.namespace_key(),
        NamespaceObject::Mount(image.path_workspace_a()),
        Operation::Observe,
    ) && event_matches(
        third_mutation,
        EventKind::NamespaceEntryResolved,
        image.path_workspace_a(),
        image.path_capability_a(),
        image.path_entry_a(),
        image.path_key_a(),
        NamespaceObject::Mount(image.path_workspace_b()),
        Operation::Observe,
    ) && event_matches(
        terminal_rebound,
        EventKind::NamespaceEntryRebound,
        image.path_workspace_b(),
        image.path_capability_b(),
        image.path_entry_b(),
        image.path_key_b(),
        NamespaceObject::Resource(image.resource()),
        Operation::Act,
    )
}

fn find_entry(
    entries: &[agent_kernel_core::NamespaceEntryRecord],
    id: NamespaceEntryId,
) -> Option<agent_kernel_core::NamespaceEntryRecord> {
    entries.iter().find(|record| record.id == id).copied()
}

fn entry_matches(
    record: agent_kernel_core::NamespaceEntryRecord,
    namespace: ResourceId,
    capability: agent_kernel_core::CapabilityId,
    key: NamespaceKey,
    object: NamespaceObject,
    revision: u64,
) -> bool {
    record.owner == RESOURCE_MANAGER
        && record.namespace == namespace
        && record.capability == capability
        && record.key == key
        && record.object == object
        && record.revision == revision
}

fn workspace_valid(booted: &X86BootedKernel, id: ResourceId) -> bool {
    booted.kernel().resources().iter().any(|resource| {
        resource.id == id
            && resource.kind == ResourceKind::Workspace
            && resource.owner == Some(RESOURCE_MANAGER)
            && resource.status == ResourceStatus::Active
    })
}

fn event_matches(
    event: &Event,
    kind: EventKind,
    resource: ResourceId,
    capability: agent_kernel_core::CapabilityId,
    entry: NamespaceEntryId,
    key: NamespaceKey,
    object: NamespaceObject,
    operation: Operation,
) -> bool {
    event.kind == kind
        && event.agent == RESOURCE_MANAGER
        && event.resource == Some(resource)
        && event.capability == Some(capability)
        && event.namespace_entry == Some(entry)
        && event.namespace_key == Some(key)
        && event.namespace_object == Some(object)
        && event.operation == Some(operation)
}
