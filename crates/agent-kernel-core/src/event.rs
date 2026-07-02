//! Kernel event records.
//!
//! This module owns the replayable event shape for observations, actions,
//! capability lifecycle changes, verification requests, checkpoints, rollback
//! requests, delegation, and scheduler decisions.

use crate::{
    ActionId, AgentId, CapabilityId, CheckpointId, Operation, OperationSet, ResourceId, TaskId,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    CapabilityGranted,
    CapabilityDerived,
    CapabilityRevoked,
    Observation,
    ActionExecuted,
    VerificationRequested,
    CheckpointCreated,
    RollbackRequested,
    DelegationRequested,
    TaskCreated,
    TaskAccepted,
    TaskCompleted,
    TaskVerified,
    TaskCancelled,
    TaskQueued,
    TaskDispatched,
    TaskYielded,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub agent: AgentId,
    pub kind: EventKind,
    pub resource: Option<ResourceId>,
    pub capability: Option<CapabilityId>,
    pub source_capability: Option<CapabilityId>,
    pub action: Option<ActionId>,
    pub operation: Option<Operation>,
    pub operations: OperationSet,
    pub checkpoint: Option<CheckpointId>,
    pub task: Option<TaskId>,
    pub target_agent: Option<AgentId>,
}

impl Event {
    pub(crate) const fn empty() -> Self {
        Self {
            sequence: 0,
            agent: AgentId::new(0),
            kind: EventKind::Observation,
            resource: None,
            capability: None,
            source_capability: None,
            action: None,
            operation: None,
            operations: OperationSet::empty(),
            checkpoint: None,
            task: None,
            target_agent: None,
        }
    }
}
