//! Kernel-owned agent launch entry descriptors.
//!
//! This module defines the fixed-width record that represents an active agent
//! being admitted into a resource-scoped runtime entry.

use crate::{AgentId, CapabilityId, IntentId, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentEntryKind {
    Bootstrap,
    Supervisor,
    Worker,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentEntryRecord {
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
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
            kind: AgentEntryKind::Worker,
            intent: None,
            task: None,
        }
    }
}
