//! Kernel-owned task model.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable task state
//! for the fixed-capacity no_std task store. It has no host dependencies and no
//! allocation.

use crate::{AgentId, CapabilityId, FaultId, IntentId, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TaskResult {
    pub code: u16,
    pub value: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Created,
    Delegated,
    Accepted,
    Running,
    Waiting,
    Faulted,
    Completed,
    Verified,
    Cancelled,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub intent: IntentId,
    pub owner: AgentId,
    pub resource: ResourceId,
    pub assignee: Option<AgentId>,
    pub delegated_capability: Option<CapabilityId>,
    pub status: TaskStatus,
    pub run_ticks: u64,
    pub quantum_remaining: u64,
    pub last_fault: Option<FaultId>,
    pub result: Option<TaskResult>,
}

impl Task {
    pub(crate) const fn empty() -> Self {
        Self {
            id: TaskId::new(0),
            intent: IntentId::new(0),
            owner: AgentId::new(0),
            resource: ResourceId::new(0),
            assignee: None,
            delegated_capability: None,
            status: TaskStatus::Cancelled,
            run_ticks: 0,
            quantum_remaining: 0,
            last_fault: None,
            result: None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TaskCompaction {
    first: TaskId,
    through: TaskId,
    count: usize,
}

impl TaskCompaction {
    pub(crate) const fn new(first: TaskId, through: TaskId, count: usize) -> Self {
        Self {
            first,
            through,
            count,
        }
    }

    pub const fn first(self) -> TaskId {
        self.first
    }

    pub const fn through(self) -> TaskId {
        self.through
    }

    pub const fn count(self) -> usize {
        self.count
    }
}
