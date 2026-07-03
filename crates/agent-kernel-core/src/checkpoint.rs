//! Kernel-owned checkpoint records.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable checkpoint
//! records for the fixed-capacity no_std checkpoint store. It does not snapshot
//! or restore resource state.

use crate::{AgentId, CapabilityId, CheckpointId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CheckpointStatus {
    Created,
    RollbackRequested,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CheckpointRecord {
    pub id: CheckpointId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub status: CheckpointStatus,
}

impl CheckpointRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: CheckpointId::new(0),
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            capability: CapabilityId::new(0),
            status: CheckpointStatus::Created,
        }
    }
}
