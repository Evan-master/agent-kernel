//! Kernel-owned fault handler records.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable handler
//! bindings used by the fixed-capacity fault handler store. Handlers bind
//! resource-scoped fault kinds to active agent IDs without host callbacks.

use crate::{AgentId, FaultHandlerId, FaultKind, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FaultHandlerRecord {
    pub id: FaultHandlerId,
    pub resource: ResourceId,
    pub kind: FaultKind,
    pub installer: AgentId,
    pub handler: AgentId,
}

impl FaultHandlerRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: FaultHandlerId::new(0),
            resource: ResourceId::new(0),
            kind: FaultKind::ExecutionTrap,
            installer: AgentId::new(0),
            handler: AgentId::new(0),
        }
    }
}
