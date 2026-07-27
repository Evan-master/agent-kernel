//! Authorized read-only Event snapshot descriptor.
//!
//! This core-layer value binds one immutable archive proposal to the
//! Supervisor, root Rollback capability, and root Resource that authorized
//! inspection. It carries no durability claim and releases no resident Event.

use crate::{AgentId, CapabilityId, EventArchiveProposal, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EventArchiveSnapshot {
    proposal: EventArchiveProposal,
    actor: AgentId,
    authority: CapabilityId,
    root: ResourceId,
}

impl EventArchiveSnapshot {
    pub(crate) const fn new(
        proposal: EventArchiveProposal,
        actor: AgentId,
        authority: CapabilityId,
        root: ResourceId,
    ) -> Self {
        Self {
            proposal,
            actor,
            authority,
            root,
        }
    }

    pub const fn proposal(self) -> EventArchiveProposal {
        self.proposal
    }

    pub const fn actor(self) -> AgentId {
        self.actor
    }

    pub const fn authority(self) -> CapabilityId {
        self.authority
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }
}
