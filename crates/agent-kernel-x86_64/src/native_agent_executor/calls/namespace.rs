//! Audited native handlers for Namespace bind, resolve, and rebind calls.

use agent_kernel_core::{
    CapabilityId, EventKind, NamespaceEntryId, NamespaceKey, NamespaceObject, Operation, ResourceId,
};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel,
};

pub(super) fn bind(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    namespace: ResourceId,
    key: NamespaceKey,
    object: NamespaceObject,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let entry_count = booted.kernel().namespace_entries().len();
    let entry = booted
        .kernel_mut()
        .sys_bind_namespace_entry(context.agent(), authority, namespace, key, object)
        .ok()?;
    let kernel = booted.kernel();
    let record = kernel
        .namespace_entries()
        .iter()
        .find(|record| record.id == entry)
        .copied()?;
    let event = kernel.events().get(event_start)?;
    if kernel.namespace_entries().len() != entry_count + 1
        || record.owner != context.agent()
        || record.namespace != namespace
        || record.capability != authority
        || record.key != key
        || record.object != object
        || record.revision != 1
        || !valid_event(
            event,
            next_sequence,
            EventKind::NamespaceEntryBound,
            context.agent(),
            authority,
            record,
            Operation::Act,
        )
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_NAMESPACE_BIND_OK");
    pending.acknowledge_namespace_binding(record)
}

pub(super) fn resolve(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    namespace: ResourceId,
    key: NamespaceKey,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let record = find_by_key(booted, namespace, key)?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let entry_count = booted.kernel().namespace_entries().len();
    let object = booted
        .kernel_mut()
        .sys_resolve_namespace_entry(context.agent(), authority, namespace, key)
        .ok()?;
    let kernel = booted.kernel();
    let retained = find_by_key(booted, namespace, key)?;
    let event = kernel.events().get(event_start)?;
    if object != record.object
        || retained != record
        || kernel.namespace_entries().len() != entry_count
        || !valid_event(
            event,
            next_sequence,
            EventKind::NamespaceEntryResolved,
            context.agent(),
            authority,
            record,
            Operation::Observe,
        )
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_NAMESPACE_RESOLVE_OK");
    pending.acknowledge_namespace_resolution(record)
}

pub(super) fn rebind(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    entry: NamespaceEntryId,
    object: NamespaceObject,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let previous = find_by_id(booted, entry)?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let entry_count = booted.kernel().namespace_entries().len();
    let event = booted
        .kernel_mut()
        .sys_rebind_namespace_entry(context.agent(), authority, entry, object)
        .ok()?;
    let kernel = booted.kernel();
    let record = find_by_id(booted, entry)?;
    if record.id != previous.id
        || record.owner != previous.owner
        || record.namespace != previous.namespace
        || record.capability != previous.capability
        || record.key != previous.key
        || record.object != object
        || record.revision != previous.revision.checked_add(1)?
        || kernel.namespace_entries().len() != entry_count
        || event != *kernel.events().get(event_start)?
        || !valid_event(
            &event,
            next_sequence,
            EventKind::NamespaceEntryRebound,
            context.agent(),
            authority,
            record,
            Operation::Act,
        )
        || kernel.events().len() != event_start + 1
        || kernel.next_event_sequence() != next_sequence.checked_add(1)?
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_NAMESPACE_REBIND_OK");
    pending.acknowledge_namespace_rebinding(record)
}

fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}

fn find_by_key(
    booted: &X86BootedKernel,
    namespace: ResourceId,
    key: NamespaceKey,
) -> Option<agent_kernel_core::NamespaceEntryRecord> {
    booted
        .kernel()
        .namespace_entries()
        .iter()
        .find(|record| record.namespace == namespace && record.key == key)
        .copied()
}

fn find_by_id(
    booted: &X86BootedKernel,
    entry: NamespaceEntryId,
) -> Option<agent_kernel_core::NamespaceEntryRecord> {
    booted
        .kernel()
        .namespace_entries()
        .iter()
        .find(|record| record.id == entry)
        .copied()
}

fn valid_event(
    event: &agent_kernel_core::Event,
    sequence: u64,
    kind: EventKind,
    actor: agent_kernel_core::AgentId,
    authority: CapabilityId,
    record: agent_kernel_core::NamespaceEntryRecord,
    operation: Operation,
) -> bool {
    event.sequence == sequence
        && event.kind == kind
        && event.agent == actor
        && event.resource == Some(record.namespace)
        && event.capability == Some(authority)
        && event.namespace_entry == Some(record.id)
        && event.namespace_key == Some(record.key)
        && event.namespace_object == Some(record.object)
        && event.operation == Some(operation)
}
