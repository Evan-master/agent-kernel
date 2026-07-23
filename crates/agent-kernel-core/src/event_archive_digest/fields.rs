//! Canonical field encoding for one Event Archive record.
//!
//! This child owns the frozen Event field order, option presence bytes, enum
//! tags, and little-endian primitive encoding shared by hash and byte sinks.

use crate::{AgentImageSignerEvent, Event, NamespaceObject};

use super::{tags, ArchiveSink};

pub(super) fn put_event(hash: &mut impl ArchiveSink, event: &Event) {
    put_u64(hash, event.sequence);
    put_u64(hash, event.agent.raw());
    put_u16(hash, tags::event_kind(event.kind));
    put_option_u64(hash, event.resource.map(|value| value.raw()));
    put_option_u64(hash, event.capability.map(|value| value.raw()));
    put_option_u64(hash, event.source_capability.map(|value| value.raw()));
    put_option_u64(hash, event.intent.map(|value| value.raw()));
    put_option_u16(hash, event.intent_kind.map(tags::intent_kind));
    put_option_u64(hash, event.action.map(|value| value.raw()));
    put_option_u64(hash, event.observation.map(|value| value.raw()));
    put_option_u64(hash, event.message.map(|value| value.raw()));
    put_option_u16(hash, event.message_kind.map(tags::message_kind));
    put_option_u64(hash, event.memory_cell.map(|value| value.raw()));
    put_option_u64(hash, event.namespace_entry.map(|value| value.raw()));
    put_option_u64(hash, event.namespace_key.map(|value| value.raw()));
    put_namespace_object(hash, event.namespace_object);
    put_option_u16(hash, event.operation.map(tags::operation));
    put_u16(hash, event.operations.bits());
    put_u16(hash, tags::verification(event.verification));
    put_option_u64(hash, event.checkpoint.map(|value| value.raw()));
    put_option_u64(hash, event.task.map(|value| value.raw()));
    put_option_u64(hash, event.runtime_admission.map(|value| value.raw()));
    put_option_pair(
        hash,
        event.task_result.map(|value| (value.code, value.value)),
    );
    put_option_u64(hash, event.task_ticks);
    put_option_u64(hash, event.task_quantum);
    put_option_u64(hash, event.fault.map(|value| value.raw()));
    put_option_u16(hash, event.fault_kind.map(tags::fault_kind));
    put_option_u64(hash, event.fault_detail);
    put_option_u64(hash, event.fault_policy.map(|value| value.raw()));
    put_option_u16(
        hash,
        event.fault_policy_action.map(tags::fault_policy_action),
    );
    put_option_u64(hash, event.waiter.map(|value| value.raw()));
    put_option_u16(hash, event.waiter_kind.map(tags::waiter_kind));
    put_option_u64(hash, event.signal.map(|value| value.raw()));
    put_option_u64(hash, event.target_agent.map(|value| value.raw()));
    put_option_u64(hash, event.driver_binding.map(|value| value.raw()));
    put_option_u64(hash, event.device_event.map(|value| value.raw()));
    put_option_u16(hash, event.device_event_kind.map(tags::device_event_kind));
    put_option_pair(
        hash,
        event
            .device_event_payload
            .map(|value| (value.code, value.value)),
    );
    put_option_u64(hash, event.driver_command.map(|value| value.raw()));
    put_option_u16(
        hash,
        event.driver_command_kind.map(tags::driver_command_kind),
    );
    put_option_pair(
        hash,
        event
            .driver_command_payload
            .map(|value| (value.opcode, value.value)),
    );
    put_option_pair(
        hash,
        event
            .driver_command_result
            .map(|value| (value.code, value.value)),
    );
    put_option_u64(hash, event.driver_invocation.map(|value| value.raw()));
    put_option_u64(hash, event.driver_invocation_ticks);
    put_option_u64(hash, event.driver_invocation_quantum);
    put_option_u64(hash, event.agent_image.map(|value| value.raw()));
    put_option_u16(hash, event.agent_image_kind.map(tags::agent_image_kind));
    match event.agent_image_digest {
        Some(value) => {
            put_u8(hash, 1);
            hash.update(value.bytes);
        }
        None => put_u8(hash, 0),
    }
    put_option_u16(hash, event.agent_image_abi_version);
    put_option_u16(hash, event.agent_image_entry_version);
    put_agent_image_signer(hash, event.agent_image_signer);
}

fn put_agent_image_signer(hash: &mut impl ArchiveSink, evidence: Option<AgentImageSignerEvent>) {
    let Some(evidence) = evidence else {
        put_u8(hash, 0);
        return;
    };
    put_u8(hash, 1);
    hash.update(evidence.signer_id.bytes());
    match evidence.peer_signer_id {
        Some(peer) => {
            put_u8(hash, 1);
            hash.update(peer.bytes());
        }
        None => put_u8(hash, 0),
    }
    hash.update(evidence.public_key);
    put_u16(hash, evidence.image_kinds.bits());
    put_u16(hash, evidence.minimum_abi);
    put_u16(hash, evidence.maximum_abi);
    put_u16(hash, tags::agent_image_signer_status(evidence.status));
    put_u64(hash, evidence.policy_generation);
}

fn put_namespace_object(hash: &mut impl ArchiveSink, object: Option<NamespaceObject>) {
    let Some(object) = object else {
        put_u8(hash, 0);
        return;
    };
    put_u8(hash, 1);
    let (tag, raw) = match object {
        NamespaceObject::Agent(value) => (1, value.raw()),
        NamespaceObject::Resource(value) => (2, value.raw()),
        NamespaceObject::Task(value) => (3, value.raw()),
        NamespaceObject::Message(value) => (4, value.raw()),
        NamespaceObject::MemoryCell(value) => (5, value.raw()),
        NamespaceObject::Mount(value) => (6, value.raw()),
    };
    put_u8(hash, tag);
    put_u64(hash, raw);
}

fn put_option_pair(hash: &mut impl ArchiveSink, value: Option<(u16, u64)>) {
    match value {
        Some((code, payload)) => {
            put_u8(hash, 1);
            put_u16(hash, code);
            put_u64(hash, payload);
        }
        None => put_u8(hash, 0),
    }
}

fn put_option_u16(hash: &mut impl ArchiveSink, value: Option<u16>) {
    match value {
        Some(value) => {
            put_u8(hash, 1);
            put_u16(hash, value);
        }
        None => put_u8(hash, 0),
    }
}

fn put_option_u64(hash: &mut impl ArchiveSink, value: Option<u64>) {
    match value {
        Some(value) => {
            put_u8(hash, 1);
            put_u64(hash, value);
        }
        None => put_u8(hash, 0),
    }
}

fn put_u8(hash: &mut impl ArchiveSink, value: u8) {
    hash.update([value]);
}

fn put_u16(hash: &mut impl ArchiveSink, value: u16) {
    hash.update(value.to_le_bytes());
}

fn put_u64(hash: &mut impl ArchiveSink, value: u64) {
    hash.update(value.to_le_bytes());
}
