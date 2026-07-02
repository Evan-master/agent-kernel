//! Kernel event records.
//!
//! This module owns the replayable event shape for observations, actions,
//! verification requests, checkpoints, rollback requests, and delegation.

use crate::{AgentId, CapabilityId, CheckpointId, Operation, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    Observation,
    ActionExecuted,
    VerificationRequested,
    CheckpointCreated,
    RollbackRequested,
    DelegationRequested,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub agent: AgentId,
    pub kind: EventKind,
    pub resource: Option<ResourceId>,
    pub capability: Option<CapabilityId>,
    pub operation: Option<Operation>,
    pub checkpoint: Option<CheckpointId>,
}

impl Event {
    pub(crate) const fn empty() -> Self {
        Self {
            sequence: 0,
            agent: AgentId::new(0),
            kind: EventKind::Observation,
            resource: None,
            capability: None,
            operation: None,
            checkpoint: None,
        }
    }
}
