#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{CapabilityId, DurableArchiveAnchor};
use agent_kernel_x86_64::{
    durable_archive_request::{
        encode_unsigned_durable_archive_request, DurableArchiveRequest,
        DurableArchiveRequestDecodeError, DurableArchiveRequestEncodeError,
        DURABLE_ARCHIVE_REQUEST_BYTES, DURABLE_ARCHIVE_REQUEST_FORMAT_VERSION,
        DURABLE_ARCHIVE_REQUEST_MAGIC, DURABLE_ARCHIVE_REQUEST_MANIFEST_OFFSET,
        DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET, DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET,
    },
    durable_state::{
        encode_durable_archive_manifest, DurableArchiveManifestDecodeError,
        DURABLE_ARCHIVE_MANIFEST_BYTES,
    },
};

use durable_state_support::{manifest, signature, signing_key, POLICY_GENERATION, ROOT, STORAGE};

const GENERATION: u64 = 9;
const STORAGE_AUTHORITY: CapabilityId = CapabilityId::new(17);

#[test]
fn kernel_stages_one_canonical_unsigned_request() {
    let key = signing_key(0x80);
    let manifest = manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );

    let bytes =
        encode_unsigned_durable_archive_request(GENERATION, STORAGE_AUTHORITY, manifest).unwrap();
    let decoded = DurableArchiveRequest::decode(&bytes, GENERATION).unwrap();

    assert_eq!(DURABLE_ARCHIVE_REQUEST_MANIFEST_OFFSET, 32);
    assert_eq!(DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET, 317);
    assert_eq!(DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET, 381);
    assert_eq!(decoded.generation(), GENERATION);
    assert_eq!(decoded.storage_authority(), STORAGE_AUTHORITY);
    assert_eq!(decoded.manifest(), manifest);
    assert_eq!(decoded.signature().bytes(), [0; 64]);
    assert_eq!(
        &bytes[DURABLE_ARCHIVE_REQUEST_MANIFEST_OFFSET..DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET],
        encode_durable_archive_manifest(manifest).as_slice()
    );
    assert_eq!(
        &bytes[DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET..DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET],
        &[0; 64]
    );
    assert_eq!(
        encode_unsigned_durable_archive_request(0, STORAGE_AUTHORITY, manifest),
        Err(DurableArchiveRequestEncodeError::ZeroGeneration)
    );
    assert_eq!(
        encode_unsigned_durable_archive_request(GENERATION, CapabilityId::new(0), manifest),
        Err(DurableArchiveRequestEncodeError::ZeroStorageAuthority)
    );
}

#[test]
fn request_decodes_one_frozen_384_byte_record() {
    let key = signing_key(0x81);
    let manifest = manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signature = signature(&key, manifest);
    let bytes = request_bytes(GENERATION, STORAGE_AUTHORITY, manifest, signature.bytes());

    let request = DurableArchiveRequest::decode(&bytes, GENERATION).expect("canonical request");

    assert_eq!(DURABLE_ARCHIVE_REQUEST_BYTES, 384);
    assert_eq!(DURABLE_ARCHIVE_REQUEST_MAGIC, *b"AKDARQ15");
    assert_eq!(DURABLE_ARCHIVE_REQUEST_FORMAT_VERSION, 1);
    assert_eq!(request.generation(), GENERATION);
    assert_eq!(request.storage_authority(), STORAGE_AUTHORITY);
    assert_eq!(request.manifest(), manifest);
    assert_eq!(request.signature(), signature);
    assert_eq!(
        &bytes[32..32 + DURABLE_ARCHIVE_MANIFEST_BYTES],
        encode_durable_archive_manifest(manifest).as_slice()
    );
    assert_eq!(&bytes[381..], &[0; 3]);
}

#[test]
fn envelope_rejects_stale_or_noncanonical_fields() {
    let key = signing_key(0x82);
    let manifest = manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signature = signature(&key, manifest);
    let canonical = request_bytes(GENERATION, STORAGE_AUTHORITY, manifest, signature.bytes());

    assert_eq!(
        DurableArchiveRequest::decode(&canonical, GENERATION + 1),
        Err(DurableArchiveRequestDecodeError::GenerationMismatch {
            expected: GENERATION + 1,
            actual: GENERATION,
        })
    );

    for (offset, expected) in [
        (0, DurableArchiveRequestDecodeError::InvalidMagic),
        (
            8,
            DurableArchiveRequestDecodeError::UnsupportedVersion { version: 0 },
        ),
        (
            10,
            DurableArchiveRequestDecodeError::UnsupportedFlags { flags: 1 },
        ),
        (
            12,
            DurableArchiveRequestDecodeError::InvalidTotalLength { length: 385 },
        ),
        (24, DurableArchiveRequestDecodeError::ZeroStorageAuthority),
        (381, DurableArchiveRequestDecodeError::ReservedNotZero),
    ] {
        let mut bytes = canonical;
        match offset {
            0 => bytes[0] = 0,
            8 => bytes[8..10].copy_from_slice(&0_u16.to_le_bytes()),
            10 => bytes[10..12].copy_from_slice(&1_u16.to_le_bytes()),
            12 => bytes[12..16].copy_from_slice(&385_u32.to_le_bytes()),
            24 => bytes[24..32].copy_from_slice(&0_u64.to_le_bytes()),
            381 => bytes[381] = 1,
            _ => unreachable!(),
        }
        assert_eq!(
            DurableArchiveRequest::decode(&bytes, GENERATION),
            Err(expected)
        );
    }
}

#[test]
fn embedded_manifest_must_remain_canonical() {
    let key = signing_key(0x83);
    let manifest = manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let mut bytes = request_bytes(
        GENERATION,
        STORAGE_AUTHORITY,
        manifest,
        signature(&key, manifest).bytes(),
    );
    bytes[32 + 33] = 1;

    assert_eq!(
        DurableArchiveRequest::decode(&bytes, GENERATION),
        Err(DurableArchiveRequestDecodeError::Manifest(
            DurableArchiveManifestDecodeError::ReservedNotZero
        ))
    );
}

fn request_bytes(
    generation: u64,
    storage_authority: CapabilityId,
    manifest: agent_kernel_core::DurableArchiveManifest,
    signature: [u8; 64],
) -> [u8; DURABLE_ARCHIVE_REQUEST_BYTES] {
    let mut bytes = [0; DURABLE_ARCHIVE_REQUEST_BYTES];
    bytes[..8].copy_from_slice(&DURABLE_ARCHIVE_REQUEST_MAGIC);
    bytes[8..10].copy_from_slice(&DURABLE_ARCHIVE_REQUEST_FORMAT_VERSION.to_le_bytes());
    bytes[12..16].copy_from_slice(&(DURABLE_ARCHIVE_REQUEST_BYTES as u32).to_le_bytes());
    bytes[16..24].copy_from_slice(&generation.to_le_bytes());
    bytes[24..32].copy_from_slice(&storage_authority.raw().to_le_bytes());
    bytes[32..317].copy_from_slice(&encode_durable_archive_manifest(manifest));
    bytes[317..381].copy_from_slice(&signature);
    bytes
}
