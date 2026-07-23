//! Errors surfaced while scanning and selecting durable slots.

use agent_kernel_core::{DurableArchiveReceiptError, DurableRecoveryError, DurableSlot};
use agent_kernel_hal::DurableStateBackendError;

use crate::durable_state::{DurableArchiveCapsuleError, DurableStateVerificationError};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveRecoveryError {
    ZeroStorageResource,
    ScratchLengthMismatch {
        length: usize,
        required: usize,
    },
    Backend {
        slot: DurableSlot,
        error: DurableStateBackendError,
    },
    ReadbackMetadataMismatch {
        slot: DurableSlot,
    },
    Capsule {
        slot: DurableSlot,
        error: DurableArchiveCapsuleError,
    },
    ExpectedCommitted {
        slot: DurableSlot,
    },
    Trust {
        slot: DurableSlot,
        error: DurableStateVerificationError,
    },
    ManifestDigestMismatch {
        slot: DurableSlot,
    },
    Receipt {
        slot: DurableSlot,
        error: DurableArchiveReceiptError,
    },
    Selection(DurableRecoveryError),
}
