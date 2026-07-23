//! One-shot machine proof produced only by a completed storage transaction.

use agent_kernel_core::{
    DurableArchiveManifest, DurableArchiveReceipt, DurableArchiveVerificationError,
    DurableArchiveVerificationRequest, DurableArchiveVerifier,
};

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedDurableArchiveCommit {
    manifest: DurableArchiveManifest,
    receipt: DurableArchiveReceipt,
    consumed: bool,
}

impl VerifiedDurableArchiveCommit {
    pub(super) const fn new(
        manifest: DurableArchiveManifest,
        receipt: DurableArchiveReceipt,
    ) -> Self {
        Self {
            manifest,
            receipt,
            consumed: false,
        }
    }

    pub const fn receipt(&self) -> DurableArchiveReceipt {
        self.receipt
    }

    pub const fn manifest(&self) -> DurableArchiveManifest {
        self.manifest
    }

    pub const fn is_consumed(&self) -> bool {
        self.consumed
    }
}

impl DurableArchiveVerifier for VerifiedDurableArchiveCommit {
    fn verify(
        &mut self,
        request: DurableArchiveVerificationRequest,
    ) -> Result<(), DurableArchiveVerificationError> {
        if self.consumed {
            return Err(DurableArchiveVerificationError::AlreadyConsumed);
        }
        let proposal = request.proposal();
        if self.manifest.generation() != proposal.generation()
            || self.manifest.first_sequence() != proposal.first_sequence()
            || self.manifest.through_sequence() != proposal.through_sequence()
            || usize::from(self.manifest.event_count()) != proposal.count()
            || self.manifest.previous_digest() != proposal.previous_digest()
            || self.manifest.archive_digest() != proposal.digest()
        {
            return Err(DurableArchiveVerificationError::ProposalMismatch);
        }
        if self.manifest.actor() != request.actor() {
            return Err(DurableArchiveVerificationError::ActorMismatch);
        }
        if self.manifest.archive_authority() != request.archive_authority() {
            return Err(DurableArchiveVerificationError::ArchiveAuthorityMismatch);
        }
        if self.manifest.root() != request.root() {
            return Err(DurableArchiveVerificationError::RootMismatch);
        }
        if self.receipt != request.receipt() {
            return Err(DurableArchiveVerificationError::ReceiptMismatch);
        }
        self.consumed = true;
        Ok(())
    }
}
