//! Untrusted durable-backend receipt values and structural checks.
//!
//! This Core child binds readback evidence to a deterministic slot, storage
//! Resource, archive generation, and anchor. Cryptographic verification must
//! occur before a receipt can authorize Event release.

use crate::{EventArchiveDigest, EventArchiveProposal, ResourceId};

use super::{DurableArchiveAnchor, DurableArchiveManifest, DurableSlot, DurableStateDigest};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveReceiptError {
    ZeroGeneration,
    ZeroStorageResource,
    ZeroFlushEpoch,
    SlotGenerationMismatch {
        expected: DurableSlot,
        actual: DurableSlot,
    },
    AnchorGenerationMismatch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveReceipt {
    slot: DurableSlot,
    storage: ResourceId,
    generation: u64,
    archive_digest: EventArchiveDigest,
    manifest_digest: DurableStateDigest,
    readback_digest: DurableStateDigest,
    flush_epoch: u64,
    anchor: DurableArchiveAnchor,
}

impl DurableArchiveReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slot: DurableSlot,
        storage: ResourceId,
        generation: u64,
        archive_digest: EventArchiveDigest,
        manifest_digest: DurableStateDigest,
        readback_digest: DurableStateDigest,
        flush_epoch: u64,
        anchor: DurableArchiveAnchor,
    ) -> Result<Self, DurableArchiveReceiptError> {
        let expected = DurableSlot::for_generation(generation)
            .ok_or(DurableArchiveReceiptError::ZeroGeneration)?;
        if storage.raw() == 0 {
            return Err(DurableArchiveReceiptError::ZeroStorageResource);
        }
        if slot != expected {
            return Err(DurableArchiveReceiptError::SlotGenerationMismatch {
                expected,
                actual: slot,
            });
        }
        if flush_epoch == 0 {
            return Err(DurableArchiveReceiptError::ZeroFlushEpoch);
        }
        if !anchor.precedes(generation) {
            return Err(DurableArchiveReceiptError::AnchorGenerationMismatch);
        }
        Ok(Self {
            slot,
            storage,
            generation,
            archive_digest,
            manifest_digest,
            readback_digest,
            flush_epoch,
            anchor,
        })
    }

    pub const fn slot(self) -> DurableSlot {
        self.slot
    }

    pub const fn storage(self) -> ResourceId {
        self.storage
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn archive_digest(self) -> EventArchiveDigest {
        self.archive_digest
    }

    pub const fn manifest_digest(self) -> DurableStateDigest {
        self.manifest_digest
    }

    pub const fn readback_digest(self) -> DurableStateDigest {
        self.readback_digest
    }

    pub const fn flush_epoch(self) -> u64 {
        self.flush_epoch
    }

    pub const fn anchor(self) -> DurableArchiveAnchor {
        self.anchor
    }

    pub fn matches(self, manifest: DurableArchiveManifest, digest: DurableStateDigest) -> bool {
        self.matches_manifest_values(manifest) && self.manifest_digest == digest
    }

    pub(super) fn matches_manifest_values(self, manifest: DurableArchiveManifest) -> bool {
        DurableSlot::for_generation(manifest.generation()) == Some(self.slot)
            && self.storage == manifest.storage()
            && self.generation == manifest.generation()
            && self.archive_digest == manifest.archive_digest()
            && self.anchor == manifest.anchor()
    }

    pub(crate) fn matches_proposal_values(self, proposal: EventArchiveProposal) -> bool {
        DurableSlot::for_generation(proposal.generation()) == Some(self.slot)
            && self.generation == proposal.generation()
            && self.archive_digest == proposal.digest()
            && match self.anchor.mode() {
                super::DurableAnchorMode::Unanchored => true,
                super::DurableAnchorMode::Trusted => {
                    self.anchor.generation().checked_add(1) == Some(proposal.generation())
                        && self.anchor.digest() == proposal.previous_digest()
                }
            }
    }
}
