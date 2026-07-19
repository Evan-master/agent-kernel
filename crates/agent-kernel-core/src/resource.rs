//! Agent Kernel resource descriptors.
//!
//! This module owns resource identity, classification, and parent linkage. It
//! does not perform lookup or authorization; `KernelCore` owns those stores.

use crate::{AgentId, CapabilityId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResourceStatus {
    Active,
    Retired,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Workspace,
    Memory,
    File,
    Process,
    Service,
    Network,
    Device,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Resource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub parent: Option<ResourceId>,
    pub owner: Option<AgentId>,
    pub status: ResourceStatus,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResourceCreateOutcome {
    pub resource: ResourceId,
    pub capability: CapabilityId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResourceRecordRetirement {
    record: Resource,
    actor: AgentId,
    authority: CapabilityId,
}

impl ResourceRecordRetirement {
    pub(crate) const fn new(record: Resource, actor: AgentId, authority: CapabilityId) -> Self {
        Self {
            record,
            actor,
            authority,
        }
    }

    pub const fn record(self) -> Resource {
        self.record
    }

    pub const fn resource(self) -> ResourceId {
        self.record.id
    }

    pub const fn actor(self) -> AgentId {
        self.actor
    }

    pub const fn authority(self) -> CapabilityId {
        self.authority
    }
}

impl Resource {
    pub(crate) const fn empty() -> Self {
        Self {
            id: ResourceId::new(0),
            kind: ResourceKind::Workspace,
            parent: None,
            owner: None,
            status: ResourceStatus::Retired,
        }
    }
}
