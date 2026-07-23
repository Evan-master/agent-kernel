//! Canonical encoding of durable slot header, body, and commit footer.

use agent_kernel_core::{
    DurableArchiveManifest, DurableArchiveSignature, DurableSlot, DURABLE_ARCHIVE_MANIFEST_BYTES,
    DURABLE_ARCHIVE_SIGNATURE_BYTES,
};
use agent_kernel_hal::{
    DURABLE_SLOT_BODY_BYTES, DURABLE_SLOT_FOOTER_BYTES, DURABLE_SLOT_HEADER_BYTES,
};
use sha2::{Digest, Sha256};

use super::{CAPSULE_FORMAT_VERSION, COMMITTED_STATE, FOOTER_MAGIC, HEADER_MAGIC, PREPARED_STATE};
use crate::durable_state::{durable_archive_manifest_digest, encode_durable_archive_manifest};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveCapsuleEncodeError {
    BodyBufferLengthMismatch { length: usize, required: usize },
    PayloadLengthMismatch { length: usize, manifest: usize },
    PayloadDigestMismatch,
}

pub fn encode_durable_archive_prepared_header(
    manifest: DurableArchiveManifest,
) -> [u8; DURABLE_SLOT_HEADER_BYTES] {
    let mut bytes = [0; DURABLE_SLOT_HEADER_BYTES];
    let body_length = manifest.payload_length()
        + DURABLE_ARCHIVE_MANIFEST_BYTES as u32
        + DURABLE_ARCHIVE_SIGNATURE_BYTES as u32;
    bytes[..8].copy_from_slice(HEADER_MAGIC);
    bytes[8..10].copy_from_slice(&CAPSULE_FORMAT_VERSION.to_le_bytes());
    bytes[10..12].copy_from_slice(&PREPARED_STATE.to_le_bytes());
    bytes[14] = slot_for_manifest(manifest) as u8;
    bytes[16..24].copy_from_slice(&manifest.generation().to_le_bytes());
    bytes[24..28].copy_from_slice(&body_length.to_le_bytes());
    bytes[28..32].copy_from_slice(&manifest.payload_length().to_le_bytes());
    bytes[32..34].copy_from_slice(&(DURABLE_ARCHIVE_MANIFEST_BYTES as u16).to_le_bytes());
    bytes[34..36].copy_from_slice(&(DURABLE_ARCHIVE_SIGNATURE_BYTES as u16).to_le_bytes());
    bytes
}

pub fn encode_durable_archive_body(
    payload: &[u8],
    manifest: DurableArchiveManifest,
    signature: DurableArchiveSignature,
    output: &mut [u8],
) -> Result<usize, DurableArchiveCapsuleEncodeError> {
    if output.len() != DURABLE_SLOT_BODY_BYTES {
        return Err(DurableArchiveCapsuleEncodeError::BodyBufferLengthMismatch {
            length: output.len(),
            required: DURABLE_SLOT_BODY_BYTES,
        });
    }
    if payload.len() != manifest.payload_length() as usize {
        return Err(DurableArchiveCapsuleEncodeError::PayloadLengthMismatch {
            length: payload.len(),
            manifest: manifest.payload_length() as usize,
        });
    }
    let payload_digest: [u8; 32] = Sha256::digest(payload).into();
    if payload_digest != manifest.payload_digest().bytes() {
        return Err(DurableArchiveCapsuleEncodeError::PayloadDigestMismatch);
    }

    output.fill(0);
    let manifest_start = payload.len();
    let signature_start = manifest_start + DURABLE_ARCHIVE_MANIFEST_BYTES;
    let body_length = signature_start + DURABLE_ARCHIVE_SIGNATURE_BYTES;
    output[..manifest_start].copy_from_slice(payload);
    output[manifest_start..signature_start]
        .copy_from_slice(&encode_durable_archive_manifest(manifest));
    output[signature_start..body_length].copy_from_slice(&signature.bytes());
    Ok(body_length)
}

pub fn encode_durable_archive_commit_footer(
    manifest: DurableArchiveManifest,
) -> [u8; DURABLE_SLOT_FOOTER_BYTES] {
    let mut bytes = [0; DURABLE_SLOT_FOOTER_BYTES];
    bytes[..8].copy_from_slice(FOOTER_MAGIC);
    bytes[8..10].copy_from_slice(&CAPSULE_FORMAT_VERSION.to_le_bytes());
    bytes[10..12].copy_from_slice(&COMMITTED_STATE.to_le_bytes());
    bytes[12] = slot_for_manifest(manifest) as u8;
    bytes[16..24].copy_from_slice(&manifest.generation().to_le_bytes());
    bytes[24..56].copy_from_slice(&durable_archive_manifest_digest(manifest).bytes());
    bytes
}

const fn slot_for_manifest(manifest: DurableArchiveManifest) -> DurableSlot {
    if manifest.generation() & 1 == 1 {
        DurableSlot::A
    } else {
        DurableSlot::B
    }
}
