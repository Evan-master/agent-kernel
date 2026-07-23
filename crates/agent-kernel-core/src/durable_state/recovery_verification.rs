//! Trusted verification boundary for one durable archive boot recovery.
//!
//! Core constructs the exact recovered-head request and accepts a machine
//! verifier only before any Event exists. Slot reads, chain selection, and
//! cryptography remain outside Core.

use super::DurableRecoveredHead;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveRecoveryVerificationError {
    Rejected,
    AlreadyConsumed,
    HeadMismatch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveRecoveryVerificationRequest {
    head: DurableRecoveredHead,
}

impl DurableArchiveRecoveryVerificationRequest {
    pub(crate) const fn new(head: DurableRecoveredHead) -> Self {
        Self { head }
    }

    pub const fn head(self) -> DurableRecoveredHead {
        self.head
    }
}

pub trait DurableArchiveRecoveryVerifier {
    fn verify(
        &mut self,
        request: DurableArchiveRecoveryVerificationRequest,
    ) -> Result<(), DurableArchiveRecoveryVerificationError>;
}
