//! Kernel-owned observation records.
//!
//! This module belongs to `agent-kernel-core`. It defines copyable observation
//! records for the fixed-capacity no_std observation store. It depends only on
//! typed kernel IDs and does not implement observation store behavior.

use crate::{AgentId, CapabilityId, ObservationId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObservationRecord {
    pub id: ObservationId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub capability: CapabilityId,
}

impl ObservationRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: ObservationId::new(0),
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            capability: CapabilityId::new(0),
        }
    }
}
