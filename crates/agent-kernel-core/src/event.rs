//! Kernel event records.
//!
//! This module owns the replayable event shape for observations, actions,
//! capability lifecycle changes, verification requests, checkpoints, rollback
//! requests, delegation, and scheduler decisions.

use crate::{
    ActionId, AgentId, CapabilityId, CheckpointId, IntentId, IntentKind, MemoryCellId, MessageId,
    NamespaceEntryId, NamespaceKey, NamespaceObject, ObservationId, Operation, OperationSet,
    ResourceId, TaskId, VerificationRequirement,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    AgentRegistered,
    AgentSuspended,
    AgentResumed,
    AgentRetired,
    CapabilityGranted,
    CapabilityDerived,
    CapabilityRevoked,
    IntentDeclared,
    IntentBound,
    IntentFulfilled,
    IntentCancelled,
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
    TaskTicked,
    TaskQuantumExpired,
    MessageSent,
    MessageReceived,
    MessageAcknowledged,
    MemoryCellCreated,
    MemoryCellRecalled,
    MemoryCellRemembered,
    NamespaceEntryBound,
    NamespaceEntryResolved,
    NamespaceEntryRebound,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub agent: AgentId,
    pub kind: EventKind,
    pub resource: Option<ResourceId>,
    pub capability: Option<CapabilityId>,
    pub source_capability: Option<CapabilityId>,
    pub intent: Option<IntentId>,
    pub intent_kind: Option<IntentKind>,
    pub action: Option<ActionId>,
    pub observation: Option<ObservationId>,
    pub message: Option<MessageId>,
    pub memory_cell: Option<MemoryCellId>,
    pub namespace_entry: Option<NamespaceEntryId>,
    pub namespace_key: Option<NamespaceKey>,
    pub namespace_object: Option<NamespaceObject>,
    pub operation: Option<Operation>,
    pub operations: OperationSet,
    pub verification: VerificationRequirement,
    pub checkpoint: Option<CheckpointId>,
    pub task: Option<TaskId>,
    pub task_ticks: Option<u64>,
    pub task_quantum: Option<u64>,
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
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            task_ticks: None,
            task_quantum: None,
            target_agent: None,
        }
    }
}
