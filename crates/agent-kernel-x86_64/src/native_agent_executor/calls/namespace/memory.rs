//! Fixed-page Namespace resolution and mutation handlers.
//!
//! This native executor child snapshots only after scheduled-context
//! authentication, delegates authority and mutation to the facade, and then
//! reconciles every retained record and emitted Event before ring-3 resume.

use agent_kernel_core::{
    EventKind, NamespaceEntryRecord, NamespaceObject, Operation, ResourceId,
    NAMESPACE_PATH_MAX_DEPTH,
};
use agent_kernel_x86_64::typed_call_data::CallDataMessage;

use super::{find_by_id, find_by_key, valid_event};
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    native_agent_executor::state,
    serial_write_line, X86BootedKernel,
};

pub(in crate::native_agent_executor::calls) fn resolve_path(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    root: ResourceId,
    generation: u64,
) -> Option<ResumableAgentCpu> {
    let path = pending.authenticated_namespace_path_buffer()?;
    if path.root() != root || path.generation() != generation {
        return None;
    }
    let context = pending.context();
    let segments = path.segments();
    let mut records: [Option<NamespaceEntryRecord>; NAMESPACE_PATH_MAX_DEPTH] =
        [None; NAMESPACE_PATH_MAX_DEPTH];
    let mut current = root;
    for (index, segment) in segments.iter().copied().enumerate() {
        let record = find_by_key(booted, current, segment.key())?;
        records[index] = Some(record);
        if index + 1 < segments.len() {
            let NamespaceObject::Mount(child) = record.object else {
                return None;
            };
            current = child;
        }
    }
    let terminal = records[segments.len() - 1]?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let entry_count = booted.kernel().namespace_entries().len();
    let resolution = booted
        .kernel_mut()
        .sys_resolve_namespace_path(context.agent(), root, segments)
        .ok()?;
    let kernel = booted.kernel();
    if resolution.root() != root
        || usize::from(resolution.depth()) != segments.len()
        || resolution.terminal() != terminal
        || kernel.namespace_entries().len() != entry_count
        || kernel.events().len() != event_start + segments.len()
        || kernel.next_event_sequence() != next_sequence.checked_add(segments.len() as u64)?
        || !segments
            .iter()
            .copied()
            .enumerate()
            .all(|(index, segment)| {
                let Some(record) = records[index] else {
                    return false;
                };
                find_by_id(booted, record.id) == Some(record)
                    && kernel
                        .events()
                        .get(event_start + index)
                        .is_some_and(|event| {
                            valid_event(
                                event,
                                next_sequence + index as u64,
                                EventKind::NamespaceEntryResolved,
                                context.agent(),
                                segment.authority(),
                                record,
                                Operation::Observe,
                            )
                        })
            })
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_NAMESPACE_MEMORY_PATH_OK");
    pending.acknowledge_namespace_memory_path_resolution(resolution, path)
}

pub(in crate::native_agent_executor::calls) fn compare_and_rebind_path(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    generation: u64,
) -> Option<ResumableAgentCpu> {
    let CallDataMessage::CompareAndRebindNamespacePath(message) =
        pending.authenticated_typed_call_data_message()?;
    if message.generation() != generation {
        return None;
    }
    let context = pending.context();
    let segments = message.segments();
    let mut records: [Option<NamespaceEntryRecord>; NAMESPACE_PATH_MAX_DEPTH] =
        [None; NAMESPACE_PATH_MAX_DEPTH];
    let mut current = message.root();
    for (index, segment) in segments.iter().copied().enumerate() {
        let record = find_by_key(booted, current, segment.key())?;
        records[index] = Some(record);
        if index + 1 < segments.len() {
            let NamespaceObject::Mount(child) = record.object else {
                return None;
            };
            current = child;
        }
    }
    let previous = records[segments.len() - 1]?;
    let event_start = booted.kernel().events().len();
    let next_sequence = booted.kernel().next_event_sequence();
    let entry_count = booted.kernel().namespace_entries().len();
    let receipt = booted
        .kernel_mut()
        .sys_compare_and_rebind_namespace_path(
            context.agent(),
            message.root(),
            segments,
            message.expected_revision(),
            message.replacement(),
        )
        .ok()?;
    let rebound = receipt.rebound();
    let kernel = booted.kernel();
    if receipt.root() != message.root()
        || usize::from(receipt.depth()) != segments.len()
        || receipt.previous() != previous
        || rebound.id != previous.id
        || rebound.owner != previous.owner
        || rebound.namespace != previous.namespace
        || rebound.capability != previous.capability
        || rebound.key != previous.key
        || rebound.object != message.replacement()
        || rebound.revision != message.expected_revision().checked_add(1)?
        || find_by_id(booted, rebound.id) != Some(rebound)
        || kernel.namespace_entries().len() != entry_count
        || kernel.events().len() != event_start + segments.len()
        || kernel.next_event_sequence() != next_sequence.checked_add(segments.len() as u64)?
        || !segments
            .iter()
            .copied()
            .enumerate()
            .all(|(index, segment)| {
                let Some(record) = records[index] else {
                    return false;
                };
                let terminal = index + 1 == segments.len();
                let expected = if terminal { rebound } else { record };
                find_by_id(booted, expected.id) == Some(expected)
                    && kernel
                        .events()
                        .get(event_start + index)
                        .is_some_and(|event| {
                            valid_event(
                                event,
                                next_sequence + index as u64,
                                if terminal {
                                    EventKind::NamespaceEntryRebound
                                } else {
                                    EventKind::NamespaceEntryResolved
                                },
                                context.agent(),
                                segment.authority(),
                                expected,
                                if terminal {
                                    Operation::Act
                                } else {
                                    Operation::Observe
                                },
                            )
                        })
            })
        || !state::running(booted, context)
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_AGENT_CALL_NAMESPACE_TYPED_REBIND_OK");
    pending.acknowledge_namespace_path_rebinding(receipt, message)
}
