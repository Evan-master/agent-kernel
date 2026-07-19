//! Kernel-owned task fault records.
//!
//! This module belongs to `agent-kernel-core`. It defines the no_std fault
//! model used by the fixed-capacity fault store. Faults are deterministic task
//! records, not host exceptions or panic payloads.

use crate::{AgentId, FaultId, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FaultKind {
    ExecutionTrap,
    AuthorityViolation,
    ResourceFault,
    VerificationFault,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FaultRecord {
    pub id: FaultId,
    pub task: TaskId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub kind: FaultKind,
    pub detail: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FaultCompaction {
    first: FaultId,
    through: FaultId,
    count: usize,
}

impl FaultCompaction {
    pub(crate) const fn new(first: FaultId, through: FaultId, count: usize) -> Self {
        Self {
            first,
            through,
            count,
        }
    }

    pub const fn first(self) -> FaultId {
        self.first
    }

    pub const fn through(self) -> FaultId {
        self.through
    }

    pub const fn count(self) -> usize {
        self.count
    }
}

impl FaultRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: FaultId::new(0),
            task: TaskId::new(0),
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            kind: FaultKind::ExecutionTrap,
            detail: 0,
        }
    }
}
