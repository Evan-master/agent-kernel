//! Canonical wire encoding for native Namespace object references.
//!
//! This architecture-library helper owns the compact object tag vocabulary
//! shared by register and fixed-page Agent Call transports. It performs no
//! lookup, authorization, allocation, or memory access.

use agent_kernel_core::{AgentId, MemoryCellId, MessageId, NamespaceObject, ResourceId, TaskId};

const OBJECT_TAG_MASK: u64 = 0b111;
const MAX_OBJECT_ID: u64 = u64::MAX >> 3;

pub const fn encode_namespace_object(object: NamespaceObject) -> Option<u64> {
    let (id, tag) = match object {
        NamespaceObject::Agent(id) => (id.raw(), 1),
        NamespaceObject::Resource(id) => (id.raw(), 2),
        NamespaceObject::Task(id) => (id.raw(), 3),
        NamespaceObject::Message(id) => (id.raw(), 4),
        NamespaceObject::MemoryCell(id) => (id.raw(), 5),
        NamespaceObject::Mount(id) => (id.raw(), 6),
    };
    if id == 0 || id > MAX_OBJECT_ID {
        None
    } else {
        Some((id << 3) | tag)
    }
}

pub const fn decode_namespace_object(word: u64) -> Option<NamespaceObject> {
    let id = word >> 3;
    if id == 0 {
        return None;
    }
    match word & OBJECT_TAG_MASK {
        1 => Some(NamespaceObject::Agent(AgentId::new(id))),
        2 => Some(NamespaceObject::Resource(ResourceId::new(id))),
        3 => Some(NamespaceObject::Task(TaskId::new(id))),
        4 => Some(NamespaceObject::Message(MessageId::new(id))),
        5 => Some(NamespaceObject::MemoryCell(MemoryCellId::new(id))),
        6 => Some(NamespaceObject::Mount(ResourceId::new(id))),
        _ => None,
    }
}
