//! Native kernel object namespace records.
//!
//! This module belongs to `agent-kernel-core`. It defines compact, copyable
//! namespace keys and typed object references without heap allocation, path
//! parsing, host filesystem access, or POSIX directory semantics.

use crate::{AgentId, CapabilityId, MemoryCellId, MessageId, NamespaceEntryId, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NamespaceKey(u64);

impl NamespaceKey {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NamespaceObject {
    Agent(AgentId),
    Resource(ResourceId),
    Task(TaskId),
    Message(MessageId),
    MemoryCell(MemoryCellId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NamespaceEntryRecord {
    pub id: NamespaceEntryId,
    pub owner: AgentId,
    pub namespace: ResourceId,
    pub capability: CapabilityId,
    pub key: NamespaceKey,
    pub object: NamespaceObject,
    pub revision: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NamespaceEntryRetirement {
    record: NamespaceEntryRecord,
    actor: AgentId,
    authority: CapabilityId,
}

impl NamespaceEntryRetirement {
    pub(crate) const fn new(
        record: NamespaceEntryRecord,
        actor: AgentId,
        authority: CapabilityId,
    ) -> Self {
        Self {
            record,
            actor,
            authority,
        }
    }

    pub const fn record(self) -> NamespaceEntryRecord {
        self.record
    }

    pub const fn namespace_entry(self) -> NamespaceEntryId {
        self.record.id
    }

    pub const fn actor(self) -> AgentId {
        self.actor
    }

    pub const fn authority(self) -> CapabilityId {
        self.authority
    }
}

impl NamespaceEntryRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: NamespaceEntryId::new(0),
            owner: AgentId::new(0),
            namespace: ResourceId::new(0),
            capability: CapabilityId::new(0),
            key: NamespaceKey::new(0),
            object: NamespaceObject::Agent(AgentId::new(0)),
            revision: 0,
        }
    }
}
