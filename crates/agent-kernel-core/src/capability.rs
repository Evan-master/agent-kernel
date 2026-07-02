//! Agent Kernel capability descriptors.
//!
//! This module owns the data carried by an explicit authorization grant. It is
//! intentionally passive; `KernelCore` owns validation and revocation behavior.

use crate::{AgentId, CapabilityId, OperationSet, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub id: CapabilityId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub operations: OperationSet,
    pub revoked: bool,
}
