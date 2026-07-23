//! Trusted verification boundary for one durable Event Archive commit.
//!
//! The facade supplies a verifier implementation. Core constructs the request,
//! accepts only a successful verification, and owns the unforgeable proof used
//! by the immediate state transition.

use crate::{AgentId, CapabilityId, EventArchiveProposal, ResourceId};

use super::DurableArchiveReceipt;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveVerificationError {
    Rejected,
    AlreadyConsumed,
    ProposalMismatch,
    ActorMismatch,
    ArchiveAuthorityMismatch,
    RootMismatch,
    ReceiptMismatch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveVerificationRequest {
    proposal: EventArchiveProposal,
    actor: AgentId,
    archive_authority: CapabilityId,
    storage_authority: CapabilityId,
    root: ResourceId,
    receipt: DurableArchiveReceipt,
}

impl DurableArchiveVerificationRequest {
    pub(crate) const fn new(
        proposal: EventArchiveProposal,
        actor: AgentId,
        archive_authority: CapabilityId,
        storage_authority: CapabilityId,
        root: ResourceId,
        receipt: DurableArchiveReceipt,
    ) -> Self {
        Self {
            proposal,
            actor,
            archive_authority,
            storage_authority,
            root,
            receipt,
        }
    }

    pub const fn proposal(self) -> EventArchiveProposal {
        self.proposal
    }

    pub const fn actor(self) -> AgentId {
        self.actor
    }

    pub const fn archive_authority(self) -> CapabilityId {
        self.archive_authority
    }

    pub const fn storage_authority(self) -> CapabilityId {
        self.storage_authority
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }

    pub const fn receipt(self) -> DurableArchiveReceipt {
        self.receipt
    }
}

pub trait DurableArchiveVerifier {
    fn verify(
        &mut self,
        request: DurableArchiveVerificationRequest,
    ) -> Result<(), DurableArchiveVerificationError>;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveCommitProof {
    request: DurableArchiveVerificationRequest,
}

impl DurableArchiveCommitProof {
    pub(crate) const fn new(request: DurableArchiveVerificationRequest) -> Self {
        Self { request }
    }

    pub(crate) const fn request(self) -> DurableArchiveVerificationRequest {
        self.request
    }
}
