//! Kernel-owned agent records.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable agent
//! records for the fixed-capacity no_std agent registry. It does not contain
//! prompts, model sessions, host process data, or scheduling policy.

use crate::AgentId;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentStatus {
    Active,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentRecord {
    pub id: AgentId,
    pub status: AgentStatus,
}

impl AgentRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: AgentId::new(0),
            status: AgentStatus::Active,
        }
    }
}
