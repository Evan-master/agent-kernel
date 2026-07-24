//! Kernel-owned agent launch entry descriptors.
//!
//! This module defines the fixed-width record that represents an active agent
//! being admitted into a resource-scoped runtime entry.

use crate::{AgentId, AgentImageId, AgentImageKind, CapabilityId, IntentId, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentEntryKind {
    Bootstrap,
    Supervisor,
    Worker,
    Verifier,
    FaultHandler,
    Driver,
    StateSigner,
}

impl AgentEntryKind {
    pub const fn image_kind(self) -> AgentImageKind {
        match self {
            Self::Bootstrap => AgentImageKind::Bootstrap,
            Self::Supervisor => AgentImageKind::Supervisor,
            Self::Worker => AgentImageKind::Worker,
            Self::Verifier => AgentImageKind::Verifier,
            Self::FaultHandler => AgentImageKind::FaultHandler,
            Self::Driver => AgentImageKind::Driver,
            Self::StateSigner => AgentImageKind::StateSigner,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentEntryRecord {
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub image: AgentImageId,
    pub kind: AgentEntryKind,
    pub intent: Option<IntentId>,
    pub task: Option<TaskId>,
}

impl AgentEntryRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            capability: CapabilityId::new(0),
            image: AgentImageId::new(0),
            kind: AgentEntryKind::Worker,
            intent: None,
            task: None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentEntryRetirement {
    entry: AgentEntryRecord,
}

impl AgentEntryRetirement {
    pub(crate) const fn new(entry: AgentEntryRecord) -> Self {
        Self { entry }
    }

    pub const fn entry(self) -> AgentEntryRecord {
        self.entry
    }

    pub const fn agent(self) -> AgentId {
        self.entry.agent
    }
}
