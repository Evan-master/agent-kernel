//! Kernel-owned fault policy records.
//!
//! This module belongs to `agent-kernel-core`. It defines deterministic,
//! copyable policy records and outcomes for applying resource-scoped fault
//! actions without heap allocation, host callbacks, or supervisor decisions.

use crate::{AgentId, Event, FaultKind, FaultPolicyId, MessageId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FaultPolicyAction {
    RouteToHandler,
    RecoverTask,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FaultPolicyRecord {
    pub id: FaultPolicyId,
    pub resource: ResourceId,
    pub kind: FaultKind,
    pub installer: AgentId,
    pub action: FaultPolicyAction,
}

impl FaultPolicyRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: FaultPolicyId::new(0),
            resource: ResourceId::new(0),
            kind: FaultKind::ExecutionTrap,
            installer: AgentId::new(0),
            action: FaultPolicyAction::RouteToHandler,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FaultPolicyOutcome {
    pub action: FaultPolicyAction,
    pub message: Option<MessageId>,
    pub event: Event,
}
