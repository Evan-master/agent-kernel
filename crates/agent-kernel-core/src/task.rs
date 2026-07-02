//! Kernel-owned task model.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable task state
//! for the fixed-capacity no_std task store. It has no host dependencies and no
//! allocation.

use crate::{AgentId, CapabilityId, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Created,
    Delegated,
    Accepted,
    Running,
    Completed,
    Verified,
    Cancelled,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub assignee: Option<AgentId>,
    pub delegated_capability: Option<CapabilityId>,
    pub status: TaskStatus,
}

impl Task {
    pub(crate) const fn empty() -> Self {
        Self {
            id: TaskId::new(0),
            owner: AgentId::new(0),
            resource: ResourceId::new(0),
            assignee: None,
            delegated_capability: None,
            status: TaskStatus::Cancelled,
        }
    }
}
