//! One-shot machine proof produced by verified dual-slot recovery.

use agent_kernel_core::{
    DurableArchiveRecoveryVerificationError, DurableArchiveRecoveryVerificationRequest,
    DurableArchiveRecoveryVerifier, DurableRecoveredHead,
};

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedDurableArchiveRecovery {
    head: DurableRecoveredHead,
    consumed: bool,
}

impl VerifiedDurableArchiveRecovery {
    pub(crate) const fn new(head: DurableRecoveredHead) -> Self {
        Self {
            head,
            consumed: false,
        }
    }

    pub const fn head(&self) -> DurableRecoveredHead {
        self.head
    }

    pub const fn is_consumed(&self) -> bool {
        self.consumed
    }
}

impl DurableArchiveRecoveryVerifier for VerifiedDurableArchiveRecovery {
    fn verify(
        &mut self,
        request: DurableArchiveRecoveryVerificationRequest,
    ) -> Result<(), DurableArchiveRecoveryVerificationError> {
        if self.consumed {
            return Err(DurableArchiveRecoveryVerificationError::AlreadyConsumed);
        }
        if request.head() != self.head {
            return Err(DurableArchiveRecoveryVerificationError::HeadMismatch);
        }
        self.consumed = true;
        Ok(())
    }
}
