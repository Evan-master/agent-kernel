//! Validation of complete fixed-width durable slot readback.

use agent_kernel_core::{
    DurableArchiveSignature, DurableSlot, ResourceId, DURABLE_ARCHIVE_MANIFEST_BYTES,
    DURABLE_ARCHIVE_SIGNATURE_BYTES, MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES,
};
use agent_kernel_hal::{
    DURABLE_SLOT_BODY_BYTES, DURABLE_SLOT_BYTES, DURABLE_SLOT_FOOTER_BYTES,
    DURABLE_SLOT_HEADER_BYTES,
};
use sha2::{Digest, Sha256};

use super::{
    DecodedDurableArchiveSlot, DurableArchiveCapsule, DurableArchiveCapsuleError,
    CAPSULE_FORMAT_VERSION, COMMITTED_STATE, FOOTER_MAGIC, HEADER_MAGIC, PREPARED_STATE,
};
use crate::durable_state::{decode_durable_archive_manifest, durable_archive_manifest_digest};

pub fn parse_durable_archive_slot(
    bytes: &[u8],
    storage: ResourceId,
    physical_slot: DurableSlot,
) -> Result<DecodedDurableArchiveSlot<'_>, DurableArchiveCapsuleError> {
    if bytes.len() != DURABLE_SLOT_BYTES {
        return Err(DurableArchiveCapsuleError::SlotLengthMismatch {
            length: bytes.len(),
            required: DURABLE_SLOT_BYTES,
        });
    }
    if bytes.iter().all(|byte| *byte == 0) {
        return Ok(DecodedDurableArchiveSlot::Empty);
    }

    let header = &bytes[..DURABLE_SLOT_HEADER_BYTES];
    if &header[..8] != HEADER_MAGIC {
        return Err(DurableArchiveCapsuleError::HeaderMagicMismatch);
    }
    let version = u16_at(header, 8);
    if version != CAPSULE_FORMAT_VERSION {
        return Err(DurableArchiveCapsuleError::HeaderVersionUnsupported { version });
    }
    let state = u16_at(header, 10);
    if state != PREPARED_STATE {
        return Err(DurableArchiveCapsuleError::HeaderStateInvalid { state });
    }
    let flags = u16_at(header, 12);
    if flags != 0 {
        return Err(DurableArchiveCapsuleError::HeaderFlagsUnsupported { flags });
    }
    let encoded_slot = slot_from_tag(header[14], true)?;
    if header[15] != 0 || header[36..].iter().any(|byte| *byte != 0) {
        return Err(DurableArchiveCapsuleError::HeaderReservedNotZero);
    }
    let generation = u64_at(header, 16);
    let expected_slot = DurableSlot::for_generation(generation)
        .ok_or(DurableArchiveCapsuleError::ZeroGeneration)?;
    if encoded_slot != expected_slot {
        return Err(DurableArchiveCapsuleError::GenerationSlotMismatch {
            expected: expected_slot,
            encoded: encoded_slot,
        });
    }
    if encoded_slot != physical_slot {
        return Err(DurableArchiveCapsuleError::PhysicalSlotMismatch {
            encoded: encoded_slot,
            physical: physical_slot,
        });
    }

    let body_length = u32_at(header, 24) as usize;
    let payload_length = u32_at(header, 28) as usize;
    let manifest_length = usize::from(u16_at(header, 32));
    let signature_length = usize::from(u16_at(header, 34));
    validate_lengths(
        body_length,
        payload_length,
        manifest_length,
        signature_length,
    )?;

    let body_start = DURABLE_SLOT_HEADER_BYTES;
    let manifest_start = body_start + payload_length;
    let signature_start = manifest_start + manifest_length;
    let body_end = body_start + body_length;
    let footer_start = DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES;
    let payload = &bytes[body_start..manifest_start];
    let manifest = decode_durable_archive_manifest(&bytes[manifest_start..signature_start])
        .map_err(DurableArchiveCapsuleError::Manifest)?;
    let signature = DurableArchiveSignature::new(array_at(bytes, signature_start));

    if manifest.generation() != generation {
        return Err(DurableArchiveCapsuleError::ManifestGenerationMismatch);
    }
    if manifest.storage() != storage {
        return Err(DurableArchiveCapsuleError::ManifestStorageMismatch {
            expected: storage,
            encoded: manifest.storage(),
        });
    }
    if manifest.payload_length() as usize != payload_length {
        return Err(DurableArchiveCapsuleError::ManifestPayloadLengthMismatch);
    }
    let payload_digest: [u8; 32] = Sha256::digest(payload).into();
    if payload_digest != manifest.payload_digest().bytes() {
        return Err(DurableArchiveCapsuleError::PayloadDigestMismatch);
    }
    if bytes[body_end..footer_start].iter().any(|byte| *byte != 0) {
        return Err(DurableArchiveCapsuleError::BodyPaddingNotZero);
    }

    let manifest_digest = durable_archive_manifest_digest(manifest);
    let capsule = DurableArchiveCapsule {
        slot: encoded_slot,
        payload,
        manifest,
        signature,
        manifest_digest,
    };
    let footer = &bytes[footer_start..];
    if footer.iter().all(|byte| *byte == 0) {
        return Ok(DecodedDurableArchiveSlot::Prepared(capsule));
    }
    validate_footer(footer, encoded_slot, generation, manifest_digest.bytes())?;
    Ok(DecodedDurableArchiveSlot::Committed(capsule))
}

fn validate_lengths(
    body_length: usize,
    payload_length: usize,
    manifest_length: usize,
    signature_length: usize,
) -> Result<(), DurableArchiveCapsuleError> {
    if payload_length == 0 || payload_length > MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES {
        return Err(DurableArchiveCapsuleError::PayloadLengthOutOfRange {
            length: payload_length,
            limit: MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES,
        });
    }
    if manifest_length != DURABLE_ARCHIVE_MANIFEST_BYTES {
        return Err(DurableArchiveCapsuleError::ManifestLengthMismatch {
            length: manifest_length,
            required: DURABLE_ARCHIVE_MANIFEST_BYTES,
        });
    }
    if signature_length != DURABLE_ARCHIVE_SIGNATURE_BYTES {
        return Err(DurableArchiveCapsuleError::SignatureLengthMismatch {
            length: signature_length,
            required: DURABLE_ARCHIVE_SIGNATURE_BYTES,
        });
    }
    let expected = payload_length + manifest_length + signature_length;
    if body_length != expected || body_length > DURABLE_SLOT_BODY_BYTES {
        return Err(DurableArchiveCapsuleError::BodyLengthMismatch {
            encoded: body_length,
            expected,
        });
    }
    Ok(())
}

fn validate_footer(
    footer: &[u8],
    slot: DurableSlot,
    generation: u64,
    manifest_digest: [u8; 32],
) -> Result<(), DurableArchiveCapsuleError> {
    if &footer[..8] != FOOTER_MAGIC {
        return Err(DurableArchiveCapsuleError::FooterMagicMismatch);
    }
    let version = u16_at(footer, 8);
    if version != CAPSULE_FORMAT_VERSION {
        return Err(DurableArchiveCapsuleError::FooterVersionUnsupported { version });
    }
    let state = u16_at(footer, 10);
    if state != COMMITTED_STATE {
        return Err(DurableArchiveCapsuleError::FooterStateInvalid { state });
    }
    let footer_slot = slot_from_tag(footer[12], false)?;
    if footer[13..16].iter().any(|byte| *byte != 0) || footer[56..].iter().any(|byte| *byte != 0) {
        return Err(DurableArchiveCapsuleError::FooterReservedNotZero);
    }
    if footer_slot != slot {
        return Err(DurableArchiveCapsuleError::FooterSlotMismatch);
    }
    if u64_at(footer, 16) != generation {
        return Err(DurableArchiveCapsuleError::FooterGenerationMismatch);
    }
    if array_at::<32>(footer, 24) != manifest_digest {
        return Err(DurableArchiveCapsuleError::FooterManifestDigestMismatch);
    }
    Ok(())
}

fn slot_from_tag(tag: u8, header: bool) -> Result<DurableSlot, DurableArchiveCapsuleError> {
    match tag {
        0 => Ok(DurableSlot::A),
        1 => Ok(DurableSlot::B),
        _ if header => Err(DurableArchiveCapsuleError::HeaderSlotInvalid { tag }),
        _ => Err(DurableArchiveCapsuleError::FooterSlotInvalid { tag }),
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(array_at(bytes, offset))
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(array_at(bytes, offset))
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(array_at(bytes, offset))
}

fn array_at<const N: usize>(bytes: &[u8], offset: usize) -> [u8; N] {
    let mut value = [0; N];
    value.copy_from_slice(&bytes[offset..offset + N]);
    value
}
