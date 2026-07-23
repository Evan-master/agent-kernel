//! Verified durable-head and recovery-outcome values.
//!
//! This Core child carries one manifest/receipt pair after machine verification
//! and labels the exact rollback guarantee implied by its anchor profile. Slot
//! scanning, chain selection, and cryptography remain outside this value layer.

use crate::EventArchiveDigest;

use super::{
    DurableAnchorMode, DurableArchiveManifest, DurableArchiveReceipt, DurableSlot,
    DurableStateDigest,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DurableRecoveryGuarantee {
    RollbackEvident = 1,
    RollbackResistant = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableRecoveryError {
    ReceiptMismatch,
    NoCommittedSlot,
    SplitBrain { generation: u64 },
    DisconnectedHead { generation: u64 },
    AnchorMismatch { generation: u64 },
    GenerationExhausted,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableRecoveredHead {
    manifest: DurableArchiveManifest,
    receipt: DurableArchiveReceipt,
}

impl DurableRecoveredHead {
    pub fn from_verified(
        manifest: DurableArchiveManifest,
        receipt: DurableArchiveReceipt,
    ) -> Result<Self, DurableRecoveryError> {
        if !receipt.matches_manifest_values(manifest) {
            return Err(DurableRecoveryError::ReceiptMismatch);
        }
        Ok(Self { manifest, receipt })
    }

    pub const fn manifest(self) -> DurableArchiveManifest {
        self.manifest
    }

    pub const fn receipt(self) -> DurableArchiveReceipt {
        self.receipt
    }

    pub const fn slot(self) -> DurableSlot {
        self.receipt.slot()
    }

    pub const fn generation(self) -> u64 {
        self.manifest.generation()
    }

    pub const fn through_sequence(self) -> u64 {
        self.manifest.through_sequence()
    }

    pub const fn previous_digest(self) -> EventArchiveDigest {
        self.manifest.previous_digest()
    }

    pub const fn archive_digest(self) -> EventArchiveDigest {
        self.manifest.archive_digest()
    }

    pub const fn manifest_digest(self) -> DurableStateDigest {
        self.receipt.manifest_digest()
    }

    pub const fn guarantee(self) -> DurableRecoveryGuarantee {
        match self.manifest.anchor().mode() {
            DurableAnchorMode::Unanchored => DurableRecoveryGuarantee::RollbackEvident,
            DurableAnchorMode::Trusted => DurableRecoveryGuarantee::RollbackResistant,
        }
    }
}
