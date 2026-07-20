//! Canonical SHA-256 commitment for archived Event segments.
//!
//! This Core implementation encodes every Event field explicitly and never
//! hashes Rust memory layout. Fixed tags, little-endian integers, and presence
//! bytes keep commitments deterministic across targets and compiler versions.

mod tags;

use sha2::{Digest, Sha256};

use crate::{Event, EventArchiveDigest, NamespaceObject};

const DOMAIN: &[u8] = b"AGENT-KERNEL-EVENT-ARCHIVE\0";
const FORMAT_VERSION: u16 = 1;

pub(super) fn digest(
    generation: u64,
    previous_through: u64,
    previous_digest: EventArchiveDigest,
    first: u64,
    through: u64,
    events: &[Event],
) -> EventArchiveDigest {
    let mut hash = Sha256::new();
    hash.update(DOMAIN);
    put_u16(&mut hash, FORMAT_VERSION);
    put_u64(&mut hash, generation);
    put_u64(&mut hash, previous_through);
    hash.update(previous_digest.bytes);
    put_u64(&mut hash, first);
    put_u64(&mut hash, through);
    put_u64(&mut hash, events.len() as u64);
    for event in events {
        put_event(&mut hash, event);
    }
    let output = hash.finalize();
    let mut bytes = [0; 32];
    bytes.copy_from_slice(&output);
    EventArchiveDigest::new(bytes)
}

fn put_event(hash: &mut Sha256, event: &Event) {
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
}

fn put_namespace_object(hash: &mut Sha256, object: Option<NamespaceObject>) {
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

fn put_option_pair(hash: &mut Sha256, value: Option<(u16, u64)>) {
    match value {
        Some((code, payload)) => {
            put_u8(hash, 1);
            put_u16(hash, code);
            put_u64(hash, payload);
        }
        None => put_u8(hash, 0),
    }
}

fn put_option_u16(hash: &mut Sha256, value: Option<u16>) {
    match value {
        Some(value) => {
            put_u8(hash, 1);
            put_u16(hash, value);
        }
        None => put_u8(hash, 0),
    }
}

fn put_option_u64(hash: &mut Sha256, value: Option<u64>) {
    match value {
        Some(value) => {
            put_u8(hash, 1);
            put_u64(hash, value);
        }
        None => put_u8(hash, 0),
    }
}

fn put_u8(hash: &mut Sha256, value: u8) {
    hash.update([value]);
}

fn put_u16(hash: &mut Sha256, value: u16) {
    hash.update(value.to_le_bytes());
}

fn put_u64(hash: &mut Sha256, value: u64) {
    hash.update(value.to_le_bytes());
}
