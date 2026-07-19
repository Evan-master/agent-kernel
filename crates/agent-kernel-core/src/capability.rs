//! Agent Kernel capability descriptors.
//!
//! This module owns the data carried by an explicit authorization grant. It is
//! intentionally passive; `KernelCore` owns validation and revocation behavior.

use crate::{AgentId, CapabilityId, OperationSet, ResourceId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub id: CapabilityId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub operations: OperationSet,
    pub revoked: bool,
    pub task: Option<TaskId>,
    pub parent: Option<CapabilityId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CapabilityCompaction {
    capability: CapabilityId,
}

impl CapabilityCompaction {
    pub(crate) const fn new(capability: CapabilityId) -> Self {
        Self { capability }
    }

    pub const fn capability(self) -> CapabilityId {
        self.capability
    }
}
