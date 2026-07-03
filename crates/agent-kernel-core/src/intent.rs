//! Agent-native intent descriptors.
//!
//! This module belongs to `agent-kernel-core`. It defines the typed,
//! fixed-size intent records that describe what an agent wants done without
//! storing prompts, natural language, or host-specific planning data.

use crate::{AgentId, IntentId, Operation, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IntentKind {
    Observe,
    Act,
    Verify,
    Checkpoint,
    Rollback,
}

impl IntentKind {
    pub const fn required_operation(self) -> Operation {
        match self {
            Self::Observe => Operation::Observe,
            Self::Act => Operation::Act,
            Self::Verify => Operation::Verify,
            Self::Checkpoint => Operation::Checkpoint,
            Self::Rollback => Operation::Rollback,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VerificationRequirement {
    Optional,
    Required,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IntentStatus {
    Declared,
    Bound,
    Fulfilled,
    Failed,
    Cancelled,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Intent {
    pub id: IntentId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub kind: IntentKind,
    pub status: IntentStatus,
    pub verification: VerificationRequirement,
}

impl Intent {
    pub(crate) const fn empty() -> Self {
        Self {
            id: IntentId::new(0),
            owner: AgentId::new(0),
            resource: ResourceId::new(0),
            kind: IntentKind::Act,
            status: IntentStatus::Declared,
            verification: VerificationRequirement::Optional,
        }
    }
}
