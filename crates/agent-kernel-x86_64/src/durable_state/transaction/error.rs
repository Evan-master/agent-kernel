//! Errors returned by the durable archive transaction boundary.

use agent_kernel_core::DurableArchiveReceiptError;
use agent_kernel_hal::{DurableSlotTargetError, DurableSlotWriteError, DurableStateBackendError};

use crate::durable_state::{
    DurableArchiveCapsuleEncodeError, DurableArchiveCapsuleError, DurableStateVerificationError,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveCommitError {
    ScratchLengthMismatch { length: usize, required: usize },
    Target(DurableSlotTargetError),
    Encode(DurableArchiveCapsuleEncodeError),
    Trust(DurableStateVerificationError),
    Write(DurableSlotWriteError),
    Backend(DurableStateBackendError),
    FlushTargetMismatch,
    FlushEpochNotAdvanced { previous: u64, actual: u64 },
    ReadbackMetadataMismatch,
    ReadbackEpochMismatch { expected: u64, actual: u64 },
    Capsule(DurableArchiveCapsuleError),
    ExpectedPreparedReadback,
    ExpectedCommittedReadback,
    ReadbackManifestMismatch,
    ReadbackSignatureMismatch,
    ReadbackPayloadMismatch,
    Receipt(DurableArchiveReceiptError),
}
