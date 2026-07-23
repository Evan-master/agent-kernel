//! Eight-operation durable archive commit transaction.

mod error;
mod verified;

pub use error::DurableArchiveCommitError;
pub use verified::VerifiedDurableArchiveCommit;

use agent_kernel_core::{
    DurableArchiveManifest, DurableArchiveReceipt, DurableArchiveSignature, DurableStateDigest,
};
use agent_kernel_hal::{
    DurableFlush, DurableSlotRegion, DurableSlotTarget, DurableSlotTargetError, DurableSlotWrite,
    DurableStateBackend, DURABLE_SLOT_BODY_BYTES, DURABLE_SLOT_BYTES, DURABLE_SLOT_FOOTER_BYTES,
    DURABLE_SLOT_HEADER_BYTES,
};
use sha2::{Digest, Sha256};

use super::{
    encode_durable_archive_body, encode_durable_archive_commit_footer,
    encode_durable_archive_prepared_header, parse_durable_archive_slot, DecodedDurableArchiveSlot,
    DurableArchiveCapsule, DurableStateTrustPolicy,
};

pub fn commit_durable_archive<B: DurableStateBackend>(
    backend: &mut B,
    policy: DurableStateTrustPolicy<'_>,
    payload: &[u8],
    manifest: DurableArchiveManifest,
    signature: DurableArchiveSignature,
    scratch: &mut [u8],
) -> Result<VerifiedDurableArchiveCommit, DurableArchiveCommitError> {
    if scratch.len() != DURABLE_SLOT_BYTES {
        return Err(DurableArchiveCommitError::ScratchLengthMismatch {
            length: scratch.len(),
            required: DURABLE_SLOT_BYTES,
        });
    }
    let verified = policy
        .verify(manifest, signature)
        .map_err(DurableArchiveCommitError::Trust)?;
    let slot = agent_kernel_core::DurableSlot::for_generation(manifest.generation())
        .ok_or(DurableSlotTargetError::ZeroGeneration)
        .map_err(DurableArchiveCommitError::Target)?;
    let target = DurableSlotTarget::new(manifest.storage(), slot, manifest.generation())
        .map_err(DurableArchiveCommitError::Target)?;
    let header = encode_durable_archive_prepared_header(manifest);
    let footer = encode_durable_archive_commit_footer(manifest);
    let body_start = DURABLE_SLOT_HEADER_BYTES;
    let footer_start = DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES;
    let body = &mut scratch[body_start..footer_start];
    encode_durable_archive_body(payload, manifest, signature, body)
        .map_err(DurableArchiveCommitError::Encode)?;
    debug_assert_eq!(body.len(), DURABLE_SLOT_BODY_BYTES);

    let header_write = DurableSlotWrite::new(target, DurableSlotRegion::PreparedHeader, &header)
        .map_err(DurableArchiveCommitError::Write)?;
    let body_write = DurableSlotWrite::new(target, DurableSlotRegion::Body, body)
        .map_err(DurableArchiveCommitError::Write)?;
    let footer_write = DurableSlotWrite::new(target, DurableSlotRegion::CommitFooter, &footer)
        .map_err(DurableArchiveCommitError::Write)?;

    backend
        .write(header_write)
        .map_err(DurableArchiveCommitError::Backend)?;
    let header_flush = flush_after(backend, target, 0)?;
    backend
        .write(body_write)
        .map_err(DurableArchiveCommitError::Backend)?;
    let body_flush = flush_after(backend, target, header_flush.epoch())?;
    read_and_verify(
        backend,
        target,
        body_flush.epoch(),
        ExpectedReadback::Prepared,
        policy,
        payload,
        manifest,
        signature,
        verified.manifest_digest(),
        scratch,
    )?;

    backend
        .write(footer_write)
        .map_err(DurableArchiveCommitError::Backend)?;
    let footer_flush = flush_after(backend, target, body_flush.epoch())?;
    read_and_verify(
        backend,
        target,
        footer_flush.epoch(),
        ExpectedReadback::Committed,
        policy,
        payload,
        manifest,
        signature,
        verified.manifest_digest(),
        scratch,
    )?;

    let readback_digest = DurableStateDigest::new(Sha256::digest(scratch).into());
    let receipt = DurableArchiveReceipt::new(
        target.slot(),
        target.storage(),
        target.generation(),
        manifest.archive_digest(),
        verified.manifest_digest(),
        readback_digest,
        footer_flush.epoch(),
        manifest.anchor(),
    )
    .map_err(DurableArchiveCommitError::Receipt)?;
    Ok(VerifiedDurableArchiveCommit::new(manifest, receipt))
}

fn flush_after<B: DurableStateBackend>(
    backend: &mut B,
    target: DurableSlotTarget,
    previous_epoch: u64,
) -> Result<DurableFlush, DurableArchiveCommitError> {
    let flush = backend
        .flush(target)
        .map_err(DurableArchiveCommitError::Backend)?;
    if flush.target() != target {
        return Err(DurableArchiveCommitError::FlushTargetMismatch);
    }
    if flush.epoch() <= previous_epoch {
        return Err(DurableArchiveCommitError::FlushEpochNotAdvanced {
            previous: previous_epoch,
            actual: flush.epoch(),
        });
    }
    Ok(flush)
}

#[allow(clippy::too_many_arguments)]
fn read_and_verify<B: DurableStateBackend>(
    backend: &mut B,
    target: DurableSlotTarget,
    expected_epoch: u64,
    expected_state: ExpectedReadback,
    policy: DurableStateTrustPolicy<'_>,
    payload: &[u8],
    manifest: DurableArchiveManifest,
    signature: DurableArchiveSignature,
    manifest_digest: DurableStateDigest,
    scratch: &mut [u8],
) -> Result<(), DurableArchiveCommitError> {
    let readback = backend
        .read_slot(target.storage(), target.slot(), scratch)
        .map_err(DurableArchiveCommitError::Backend)?;
    if readback.storage() != target.storage()
        || readback.slot() != target.slot()
        || readback.bytes_read() != DURABLE_SLOT_BYTES
    {
        return Err(DurableArchiveCommitError::ReadbackMetadataMismatch);
    }
    if readback.flush_epoch() != expected_epoch {
        return Err(DurableArchiveCommitError::ReadbackEpochMismatch {
            expected: expected_epoch,
            actual: readback.flush_epoch(),
        });
    }
    let decoded = parse_durable_archive_slot(scratch, target.storage(), target.slot())
        .map_err(DurableArchiveCommitError::Capsule)?;
    let capsule = match (expected_state, decoded) {
        (ExpectedReadback::Prepared, DecodedDurableArchiveSlot::Prepared(capsule))
        | (ExpectedReadback::Committed, DecodedDurableArchiveSlot::Committed(capsule)) => capsule,
        (ExpectedReadback::Prepared, _) => {
            return Err(DurableArchiveCommitError::ExpectedPreparedReadback)
        }
        (ExpectedReadback::Committed, _) => {
            return Err(DurableArchiveCommitError::ExpectedCommittedReadback)
        }
    };
    validate_readback_capsule(
        capsule,
        policy,
        payload,
        manifest,
        signature,
        manifest_digest,
    )
}

fn validate_readback_capsule(
    capsule: DurableArchiveCapsule<'_>,
    policy: DurableStateTrustPolicy<'_>,
    payload: &[u8],
    manifest: DurableArchiveManifest,
    signature: DurableArchiveSignature,
    manifest_digest: DurableStateDigest,
) -> Result<(), DurableArchiveCommitError> {
    if capsule.manifest() != manifest || capsule.manifest_digest() != manifest_digest {
        return Err(DurableArchiveCommitError::ReadbackManifestMismatch);
    }
    if capsule.signature() != signature {
        return Err(DurableArchiveCommitError::ReadbackSignatureMismatch);
    }
    if capsule.payload() != payload {
        return Err(DurableArchiveCommitError::ReadbackPayloadMismatch);
    }
    policy
        .verify(capsule.manifest(), capsule.signature())
        .map_err(DurableArchiveCommitError::Trust)?;
    Ok(())
}

#[derive(Copy, Clone)]
enum ExpectedReadback {
    Prepared,
    Committed,
}
