//! Kernel-owned action records.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable action
//! records for the fixed-capacity no_std action store. It depends only on
//! typed kernel IDs and does not implement action store behavior.

use crate::{ActionId, AgentId, CapabilityId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ActionStatus {
    Executed,
    VerificationRequested,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ActionRecord {
    pub id: ActionId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
    pub status: ActionStatus,
}

impl ActionRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: ActionId::new(0),
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            capability: CapabilityId::new(0),
            status: ActionStatus::Executed,
        }
    }
}
