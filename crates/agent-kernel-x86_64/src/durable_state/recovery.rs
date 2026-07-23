//! Dual-slot durable archive recovery and verification.

mod error;
mod select;

pub use error::DurableArchiveRecoveryError;

use agent_kernel_core::{
    DurableArchiveReceipt, DurableRecoveredHead, DurableSlot, DurableStateDigest, ResourceId,
};
use agent_kernel_hal::{DurableStateBackend, DURABLE_SLOT_BYTES, DURABLE_SLOT_FOOTER_BYTES};
use sha2::{Digest, Sha256};

use super::{parse_durable_archive_slot, DecodedDurableArchiveSlot, DurableStateTrustPolicy};
use select::select_recovered_head;

pub fn recover_durable_archive<B: DurableStateBackend>(
    backend: &mut B,
    policy: DurableStateTrustPolicy<'_>,
    storage: ResourceId,
    scratch: &mut [u8],
) -> Result<DurableRecoveredHead, DurableArchiveRecoveryError> {
    if storage.raw() == 0 {
        return Err(DurableArchiveRecoveryError::ZeroStorageResource);
    }
    if scratch.len() != DURABLE_SLOT_BYTES {
        return Err(DurableArchiveRecoveryError::ScratchLengthMismatch {
            length: scratch.len(),
            required: DURABLE_SLOT_BYTES,
        });
    }

    let slot_a = read_candidate(backend, policy, storage, DurableSlot::A, scratch)?;
    let slot_b = read_candidate(backend, policy, storage, DurableSlot::B, scratch)?;
    select_recovered_head(slot_a, slot_b).map_err(DurableArchiveRecoveryError::Selection)
}

fn read_candidate<B: DurableStateBackend>(
    backend: &mut B,
    policy: DurableStateTrustPolicy<'_>,
    storage: ResourceId,
    slot: DurableSlot,
    scratch: &mut [u8],
) -> Result<Option<DurableRecoveredHead>, DurableArchiveRecoveryError> {
    let readback = backend
        .read_slot(storage, slot, scratch)
        .map_err(|error| DurableArchiveRecoveryError::Backend { slot, error })?;
    if readback.storage() != storage
        || readback.slot() != slot
        || readback.bytes_read() != DURABLE_SLOT_BYTES
    {
        return Err(DurableArchiveRecoveryError::ReadbackMetadataMismatch { slot });
    }

    let footer_start = DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES;
    if scratch[footer_start..].iter().all(|byte| *byte == 0) {
        return Ok(None);
    }
    let decoded = parse_durable_archive_slot(scratch, storage, slot)
        .map_err(|error| DurableArchiveRecoveryError::Capsule { slot, error })?;
    let DecodedDurableArchiveSlot::Committed(capsule) = decoded else {
        return Err(DurableArchiveRecoveryError::ExpectedCommitted { slot });
    };
    let verified = policy
        .verify(capsule.manifest(), capsule.signature())
        .map_err(|error| DurableArchiveRecoveryError::Trust { slot, error })?;
    if verified.manifest_digest() != capsule.manifest_digest() {
        return Err(DurableArchiveRecoveryError::ManifestDigestMismatch { slot });
    }

    let manifest = capsule.manifest();
    let readback_digest = DurableStateDigest::new(Sha256::digest(&*scratch).into());
    let receipt = DurableArchiveReceipt::new(
        slot,
        storage,
        manifest.generation(),
        manifest.archive_digest(),
        capsule.manifest_digest(),
        readback_digest,
        readback.flush_epoch(),
        manifest.anchor(),
    )
    .map_err(|error| DurableArchiveRecoveryError::Receipt { slot, error })?;
    let head = DurableRecoveredHead::from_verified(manifest, receipt)
        .map_err(DurableArchiveRecoveryError::Selection)?;
    Ok(Some(head))
}
