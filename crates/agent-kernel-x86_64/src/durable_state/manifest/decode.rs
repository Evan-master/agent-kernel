//! Decoder for the canonical durable archive signing message.

use agent_kernel_core::{
    AgentId, CapabilityId, DurableArchiveAnchor, DurableArchiveManifest,
    DurableArchiveManifestError, DurableArchiveManifestFields, DurableStateDigest,
    DurableStateSignerId, EventArchiveDigest, ResourceId, DURABLE_ARCHIVE_MANIFEST_BYTES,
};

use super::{DOMAIN, DURABLE_ARCHIVE_MANIFEST_FORMAT_VERSION, TRUSTED_ANCHOR_FLAG};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveManifestDecodeError {
    LengthMismatch { length: usize, required: usize },
    DomainMismatch,
    UnsupportedVersion { version: u16 },
    UnsupportedFlags { flags: u16 },
    ReservedNotZero,
    AnchorEncodingInvalid,
    Manifest(DurableArchiveManifestError),
}

pub fn decode_durable_archive_manifest(
    bytes: &[u8],
) -> Result<DurableArchiveManifest, DurableArchiveManifestDecodeError> {
    if bytes.len() != DURABLE_ARCHIVE_MANIFEST_BYTES {
        return Err(DurableArchiveManifestDecodeError::LengthMismatch {
            length: bytes.len(),
            required: DURABLE_ARCHIVE_MANIFEST_BYTES,
        });
    }

    let mut decoder = ManifestDecoder::new(bytes);
    if decoder.take::<29>() != *DOMAIN {
        return Err(DurableArchiveManifestDecodeError::DomainMismatch);
    }
    let version = decoder.u16();
    if version != DURABLE_ARCHIVE_MANIFEST_FORMAT_VERSION {
        return Err(DurableArchiveManifestDecodeError::UnsupportedVersion { version });
    }
    let flags = decoder.u16();
    if flags & !TRUSTED_ANCHOR_FLAG != 0 {
        return Err(DurableArchiveManifestDecodeError::UnsupportedFlags { flags });
    }
    if !decoder.zeroes::<4>() {
        return Err(DurableArchiveManifestDecodeError::ReservedNotZero);
    }

    let generation = decoder.u64();
    let first_sequence = decoder.u64();
    let through_sequence = decoder.u64();
    let event_count = decoder.u16();
    if !decoder.zeroes::<6>() {
        return Err(DurableArchiveManifestDecodeError::ReservedNotZero);
    }
    let previous_digest = EventArchiveDigest::new(decoder.take());
    let archive_digest = EventArchiveDigest::new(decoder.take());
    let actor = AgentId::new(decoder.u64());
    let archive_authority = CapabilityId::new(decoder.u64());
    let root = ResourceId::new(decoder.u64());
    let storage = ResourceId::new(decoder.u64());
    let payload_length = decoder.u32();
    if !decoder.zeroes::<4>() {
        return Err(DurableArchiveManifestDecodeError::ReservedNotZero);
    }
    let payload_digest = DurableStateDigest::new(decoder.take());
    let signer_id = DurableStateSignerId::new(decoder.take());
    let signer_policy_generation = decoder.u64();
    let anchor_generation = decoder.u64();
    let anchor_digest = EventArchiveDigest::new(decoder.take());
    let anchor = decode_anchor(flags, anchor_generation, anchor_digest)?;

    DurableArchiveManifest::from_fields(DurableArchiveManifestFields {
        generation,
        first_sequence,
        through_sequence,
        event_count,
        previous_digest,
        archive_digest,
        actor,
        archive_authority,
        root,
        storage,
        payload_length,
        payload_digest,
        signer_id,
        signer_policy_generation,
        anchor,
    })
    .map_err(DurableArchiveManifestDecodeError::Manifest)
}

fn decode_anchor(
    flags: u16,
    generation: u64,
    digest: EventArchiveDigest,
) -> Result<DurableArchiveAnchor, DurableArchiveManifestDecodeError> {
    if flags == 0 {
        if generation != 0 || digest != EventArchiveDigest::ZERO {
            return Err(DurableArchiveManifestDecodeError::AnchorEncodingInvalid);
        }
        return Ok(DurableArchiveAnchor::unanchored());
    }
    DurableArchiveAnchor::trusted(generation, digest)
        .ok_or(DurableArchiveManifestDecodeError::AnchorEncodingInvalid)
}

struct ManifestDecoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ManifestDecoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut value = [0; N];
        let end = self.offset + N;
        value.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        value
    }

    fn zeroes<const N: usize>(&mut self) -> bool {
        self.take::<N>().iter().all(|byte| *byte == 0)
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.take())
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take())
    }

    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.take())
    }
}
