//! Canonical signed durable archive request carried by Agent call-data memory.
//!
//! This architecture-library module owns fixed-width envelope decoding. It
//! performs no Agent-memory access, authorization, signature verification,
//! allocation, or storage I/O.

use agent_kernel_core::{
    CapabilityId, DurableArchiveManifest, DurableArchiveSignature, DURABLE_ARCHIVE_MANIFEST_BYTES,
    DURABLE_ARCHIVE_SIGNATURE_BYTES,
};

use crate::durable_state::{
    decode_durable_archive_manifest, encode_durable_archive_manifest,
    DurableArchiveManifestDecodeError,
};

pub const DURABLE_ARCHIVE_REQUEST_MAGIC: [u8; 8] = *b"AKDARQ15";
pub const DURABLE_ARCHIVE_REQUEST_FORMAT_VERSION: u16 = 1;
pub const DURABLE_ARCHIVE_REQUEST_BYTES: usize = 384;
pub const DURABLE_ARCHIVE_REQUEST_MANIFEST_OFFSET: usize = 32;
pub const DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET: usize =
    DURABLE_ARCHIVE_REQUEST_MANIFEST_OFFSET + DURABLE_ARCHIVE_MANIFEST_BYTES;
pub const DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET: usize =
    DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET + DURABLE_ARCHIVE_SIGNATURE_BYTES;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveRequestEncodeError {
    ZeroGeneration,
    ZeroStorageAuthority,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveRequestDecodeError {
    InvalidMagic,
    UnsupportedVersion { version: u16 },
    UnsupportedFlags { flags: u16 },
    InvalidTotalLength { length: u32 },
    GenerationMismatch { expected: u64, actual: u64 },
    ZeroStorageAuthority,
    Manifest(DurableArchiveManifestDecodeError),
    ReservedNotZero,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveRequest {
    generation: u64,
    storage_authority: CapabilityId,
    manifest: DurableArchiveManifest,
    signature: DurableArchiveSignature,
}

pub fn encode_unsigned_durable_archive_request(
    generation: u64,
    storage_authority: CapabilityId,
    manifest: DurableArchiveManifest,
) -> Result<[u8; DURABLE_ARCHIVE_REQUEST_BYTES], DurableArchiveRequestEncodeError> {
    if generation == 0 {
        return Err(DurableArchiveRequestEncodeError::ZeroGeneration);
    }
    if storage_authority.raw() == 0 {
        return Err(DurableArchiveRequestEncodeError::ZeroStorageAuthority);
    }

    let mut bytes = [0; DURABLE_ARCHIVE_REQUEST_BYTES];
    bytes[..8].copy_from_slice(&DURABLE_ARCHIVE_REQUEST_MAGIC);
    bytes[8..10].copy_from_slice(&DURABLE_ARCHIVE_REQUEST_FORMAT_VERSION.to_le_bytes());
    bytes[12..16].copy_from_slice(&(DURABLE_ARCHIVE_REQUEST_BYTES as u32).to_le_bytes());
    bytes[16..24].copy_from_slice(&generation.to_le_bytes());
    bytes[24..32].copy_from_slice(&storage_authority.raw().to_le_bytes());
    bytes[DURABLE_ARCHIVE_REQUEST_MANIFEST_OFFSET..DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET]
        .copy_from_slice(&encode_durable_archive_manifest(manifest));
    Ok(bytes)
}

impl DurableArchiveRequest {
    pub fn decode(
        bytes: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
        expected_generation: u64,
    ) -> Result<Self, DurableArchiveRequestDecodeError> {
        if bytes[..8] != DURABLE_ARCHIVE_REQUEST_MAGIC {
            return Err(DurableArchiveRequestDecodeError::InvalidMagic);
        }
        let version = read_u16(bytes, 8);
        if version != DURABLE_ARCHIVE_REQUEST_FORMAT_VERSION {
            return Err(DurableArchiveRequestDecodeError::UnsupportedVersion { version });
        }
        let flags = read_u16(bytes, 10);
        if flags != 0 {
            return Err(DurableArchiveRequestDecodeError::UnsupportedFlags { flags });
        }
        let length = read_u32(bytes, 12);
        if length != DURABLE_ARCHIVE_REQUEST_BYTES as u32 {
            return Err(DurableArchiveRequestDecodeError::InvalidTotalLength { length });
        }
        let generation = read_u64(bytes, 16);
        if generation == 0 || generation != expected_generation {
            return Err(DurableArchiveRequestDecodeError::GenerationMismatch {
                expected: expected_generation,
                actual: generation,
            });
        }
        let storage_authority = CapabilityId::new(read_u64(bytes, 24));
        if storage_authority.raw() == 0 {
            return Err(DurableArchiveRequestDecodeError::ZeroStorageAuthority);
        }

        let mut encoded_manifest = [0; DURABLE_ARCHIVE_MANIFEST_BYTES];
        encoded_manifest.copy_from_slice(
            &bytes
                [DURABLE_ARCHIVE_REQUEST_MANIFEST_OFFSET..DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET],
        );
        let manifest = decode_durable_archive_manifest(&encoded_manifest)
            .map_err(DurableArchiveRequestDecodeError::Manifest)?;
        let mut signature = [0; DURABLE_ARCHIVE_SIGNATURE_BYTES];
        signature.copy_from_slice(
            &bytes
                [DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET..DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET],
        );
        if bytes[DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET..]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(DurableArchiveRequestDecodeError::ReservedNotZero);
        }

        Ok(Self {
            generation,
            storage_authority,
            manifest,
            signature: DurableArchiveSignature::new(signature),
        })
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn storage_authority(self) -> CapabilityId {
        self.storage_authority
    }

    pub const fn manifest(self) -> DurableArchiveManifest {
        self.manifest
    }

    pub const fn signature(self) -> DurableArchiveSignature {
        self.signature
    }
}

fn read_u16(bytes: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES], offset: usize) -> u32 {
    let mut value = [0; 4];
    value.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(value)
}

fn read_u64(bytes: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES], offset: usize) -> u64 {
    let mut value = [0; 8];
    value.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(value)
}

const _: () = assert!(DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET == 381);
