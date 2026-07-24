//! Fixed-width values shared by signed durable-state protocol layers.
//!
//! This Core module owns allocator-free digest, signature, anchor, and slot
//! identities. Child modules validate manifests, receipts, recovery heads, and
//! State Signer records without performing storage I/O or signature arithmetic.

mod manifest;
mod receipt;
mod recovery;
mod recovery_verification;
mod signer;
mod verification;

pub use manifest::{
    DurableArchiveManifest, DurableArchiveManifestError, DurableArchiveManifestFields,
    DurableArchiveManifestVersion,
};
pub use receipt::{DurableArchiveReceipt, DurableArchiveReceiptError};
pub use recovery::{DurableRecoveredHead, DurableRecoveryError, DurableRecoveryGuarantee};
pub use recovery_verification::{
    DurableArchiveRecoveryVerificationError, DurableArchiveRecoveryVerificationRequest,
    DurableArchiveRecoveryVerifier,
};
pub use signer::{
    durable_state_signer_id, durable_state_signer_id_for_key, DurableSignatureAlgorithm,
    DurableStatePublicKey, DurableStateSignerId, DurableStateSignerRecord,
    DurableStateSignerStatus,
};
pub use verification::{
    DurableArchiveCommitProof, DurableArchiveVerificationError, DurableArchiveVerificationRequest,
    DurableArchiveVerifier,
};

use crate::EventArchiveDigest;

pub const DURABLE_ARCHIVE_MANIFEST_BYTES: usize = 285;
pub const DURABLE_ARCHIVE_SIGNATURE_BYTES: usize = 64;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableStateDigest {
    bytes: [u8; 32],
}

impl DurableStateDigest {
    pub const ZERO: Self = Self { bytes: [0; 32] };

    pub const fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub const fn from_archive(digest: EventArchiveDigest) -> Self {
        Self {
            bytes: digest.bytes,
        }
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.bytes
    }

    pub const fn is_zero(self) -> bool {
        bytes_are_zero(&self.bytes)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveSignature {
    bytes: [u8; DURABLE_ARCHIVE_SIGNATURE_BYTES],
}

impl DurableArchiveSignature {
    pub const fn new(bytes: [u8; DURABLE_ARCHIVE_SIGNATURE_BYTES]) -> Self {
        Self { bytes }
    }

    pub const fn bytes(self) -> [u8; DURABLE_ARCHIVE_SIGNATURE_BYTES] {
        self.bytes
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DurableSlot {
    A = 0,
    B = 1,
}

impl DurableSlot {
    pub const fn for_generation(generation: u64) -> Option<Self> {
        if generation == 0 {
            None
        } else if generation & 1 == 1 {
            Some(Self::A)
        } else {
            Some(Self::B)
        }
    }

    pub const fn alternate(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DurableAnchorMode {
    Unanchored = 0,
    Trusted = 1,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveAnchor {
    mode: DurableAnchorMode,
    generation: u64,
    digest: EventArchiveDigest,
}

impl DurableArchiveAnchor {
    pub const fn unanchored() -> Self {
        Self {
            mode: DurableAnchorMode::Unanchored,
            generation: 0,
            digest: EventArchiveDigest::ZERO,
        }
    }

    pub const fn trusted(generation: u64, digest: EventArchiveDigest) -> Option<Self> {
        let digest_is_zero = bytes_are_zero(&digest.bytes);
        if (generation == 0) != digest_is_zero {
            return None;
        }
        Some(Self {
            mode: DurableAnchorMode::Trusted,
            generation,
            digest,
        })
    }

    pub const fn mode(self) -> DurableAnchorMode {
        self.mode
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn digest(self) -> EventArchiveDigest {
        self.digest
    }

    pub(super) fn matches_previous(
        self,
        generation: u64,
        previous_digest: EventArchiveDigest,
    ) -> bool {
        match self.mode {
            DurableAnchorMode::Unanchored => true,
            DurableAnchorMode::Trusted => {
                self.generation.checked_add(1) == Some(generation)
                    && self.digest.bytes == previous_digest.bytes
            }
        }
    }

    pub(super) fn precedes(self, generation: u64) -> bool {
        match self.mode {
            DurableAnchorMode::Unanchored => true,
            DurableAnchorMode::Trusted => self.generation.checked_add(1) == Some(generation),
        }
    }
}

const fn bytes_are_zero(bytes: &[u8]) -> bool {
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0 {
            return false;
        }
        index += 1;
    }
    true
}
