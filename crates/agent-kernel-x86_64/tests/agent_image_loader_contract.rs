use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageKind, AgentImageRecord, AgentImageStatus,
    ResourceId,
};
use agent_kernel_x86_64::agent_image::{
    sha256_digest, AgentImageCapsule, AgentImageLoadError, VerifiedAgentImage,
    AGENT_IMAGE_HEADER_BYTES, MAX_AGENT_CODE_BYTES,
};

const ABI_VERSION: u16 = 1;
const ENTRY_VERSION: u16 = 1;

#[test]
fn sha256_wrapper_matches_standard_abc_vector() {
    assert_eq!(
        sha256_digest(b"abc"),
        AgentImageDigest::new([
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ])
    );
}

#[test]
fn native_capsule_parses_exact_x86_worker_header_and_code() {
    let bytes = capsule(&[0x90, 0xcd, 0x90], 1, 1, 0, ABI_VERSION, ENTRY_VERSION, 1);
    let parsed = AgentImageCapsule::parse(&bytes).unwrap();

    assert_eq!(AGENT_IMAGE_HEADER_BYTES, 32);
    assert_eq!(parsed.header().format_version(), 1);
    assert_eq!(parsed.header().architecture(), 1);
    assert_eq!(parsed.header().image_kind(), 1);
    assert_eq!(parsed.header().abi_version(), ABI_VERSION);
    assert_eq!(parsed.header().entry_version(), ENTRY_VERSION);
    assert_eq!(parsed.entry_offset(), 1);
    assert_eq!(parsed.code(), &[0x90, 0xcd, 0x90]);
}

#[test]
fn native_capsule_rejects_unknown_or_noncanonical_header_fields() {
    let valid = capsule(&[0x90], 1, 1, 0, ABI_VERSION, ENTRY_VERSION, 0);
    let cases = [
        (mutated(&valid, 0, b'X'), AgentImageLoadError::InvalidMagic),
        (
            mutated_u16(&valid, 8, 2),
            AgentImageLoadError::UnsupportedFormatVersion,
        ),
        (
            mutated_u16(&valid, 10, 2),
            AgentImageLoadError::UnsupportedArchitecture,
        ),
        (
            mutated_u16(&valid, 12, 3),
            AgentImageLoadError::UnsupportedImageKind,
        ),
        (
            mutated_u16(&valid, 14, 1),
            AgentImageLoadError::UnsupportedFlags,
        ),
        (
            mutated_u16(&valid, 16, 0),
            AgentImageLoadError::InvalidVersion,
        ),
        (
            mutated_u32(&valid, 28, 1),
            AgentImageLoadError::ReservedNotZero,
        ),
    ];

    for (bytes, expected) in cases {
        assert_eq!(AgentImageCapsule::parse(&bytes), Err(expected));
    }
}

#[test]
fn native_verifier_capsule_binds_verifier_metadata() {
    let bytes = capsule(&[0x90, 0xcd, 0x90], 1, 2, 0, ABI_VERSION, ENTRY_VERSION, 0);
    let parsed = AgentImageCapsule::parse(&bytes).unwrap();
    assert_eq!(parsed.header().image_kind(), 2);

    let mut verifier = record(sha256_digest(&bytes), AgentImageStatus::Verified);
    verifier.kind = AgentImageKind::Verifier;
    let verified = VerifiedAgentImage::verify(verifier, &bytes).unwrap();
    assert_eq!(verified.record().kind, AgentImageKind::Verifier);
    assert_eq!(verified.code(), &[0x90, 0xcd, 0x90]);
}

#[test]
fn native_capsule_rejects_bad_lengths_and_entry_bounds() {
    assert_eq!(
        AgentImageCapsule::parse(&[0; AGENT_IMAGE_HEADER_BYTES - 1]),
        Err(AgentImageLoadError::HeaderTruncated)
    );
    let mut oversized_code = vec![0x90; MAX_AGENT_CODE_BYTES + 1];
    let oversized = capsule(&oversized_code, 1, 1, 0, ABI_VERSION, ENTRY_VERSION, 0);
    oversized_code.clear();
    assert_eq!(
        AgentImageCapsule::parse(&oversized),
        Err(AgentImageLoadError::InvalidCodeLength)
    );

    let valid = capsule(&[0x90, 0x90], 1, 1, 0, ABI_VERSION, ENTRY_VERSION, 0);
    assert_eq!(
        AgentImageCapsule::parse(&valid[..valid.len() - 1]),
        Err(AgentImageLoadError::LengthMismatch)
    );
    let mut trailing = valid.clone();
    trailing.push(0);
    assert_eq!(
        AgentImageCapsule::parse(&trailing),
        Err(AgentImageLoadError::LengthMismatch)
    );
    assert_eq!(
        AgentImageCapsule::parse(&mutated_u32(&valid, 20, 2)),
        Err(AgentImageLoadError::EntryOutOfRange)
    );
    assert_eq!(
        AgentImageCapsule::parse(&capsule(&[], 1, 1, 0, ABI_VERSION, ENTRY_VERSION, 0,)),
        Err(AgentImageLoadError::InvalidCodeLength)
    );
}

#[test]
fn verified_image_binds_kernel_record_to_exact_capsule_bytes() {
    let bytes = capsule(&[0x90, 0xcd, 0x90], 1, 1, 0, ABI_VERSION, ENTRY_VERSION, 1);
    let record = record(sha256_digest(&bytes), AgentImageStatus::Verified);
    let verified = VerifiedAgentImage::verify(record, &bytes).unwrap();

    assert_eq!(verified.record(), record);
    assert_eq!(verified.code(), &[0x90, 0xcd, 0x90]);
    assert_eq!(verified.entry_offset(), 1);

    let mut changed = bytes.clone();
    *changed.last_mut().unwrap() ^= 1;
    assert_eq!(
        VerifiedAgentImage::verify(record, &changed),
        Err(AgentImageLoadError::DigestMismatch)
    );
}

#[test]
fn verified_image_rejects_unverified_or_mismatched_metadata() {
    let bytes = capsule(&[0x90], 1, 1, 0, ABI_VERSION, ENTRY_VERSION, 0);
    let digest = sha256_digest(&bytes);

    assert_eq!(
        VerifiedAgentImage::verify(record(digest, AgentImageStatus::Pending), &bytes),
        Err(AgentImageLoadError::ImageNotVerified)
    );
    assert_eq!(
        VerifiedAgentImage::verify(record(digest, AgentImageStatus::Retired), &bytes),
        Err(AgentImageLoadError::ImageNotVerified)
    );
    let mut wrong_kind = record(digest, AgentImageStatus::Verified);
    wrong_kind.kind = AgentImageKind::Driver;
    assert_eq!(
        VerifiedAgentImage::verify(wrong_kind, &bytes),
        Err(AgentImageLoadError::MetadataMismatch)
    );
    let mut wrong_version = record(digest, AgentImageStatus::Verified);
    wrong_version.entry_version = 2;
    assert_eq!(
        VerifiedAgentImage::verify(wrong_version, &bytes),
        Err(AgentImageLoadError::MetadataMismatch)
    );
}

#[test]
fn distinct_worker_capsules_have_distinct_digests() {
    let first = capsule(&[0xcd, 0x90], 1, 1, 0, ABI_VERSION, ENTRY_VERSION, 0);
    let second = capsule(
        &[0x90, 0x90, 0xcd, 0x90],
        1,
        1,
        0,
        ABI_VERSION,
        ENTRY_VERSION,
        0,
    );

    assert_ne!(sha256_digest(&first), sha256_digest(&second));
}

fn record(digest: AgentImageDigest, status: AgentImageStatus) -> AgentImageRecord {
    AgentImageRecord {
        id: AgentImageId::new(7),
        owner: AgentId::new(1),
        resource: ResourceId::new(1),
        kind: AgentImageKind::Worker,
        digest,
        abi_version: ABI_VERSION,
        entry_version: ENTRY_VERSION,
        status,
    }
}

fn capsule(
    code: &[u8],
    architecture: u16,
    image_kind: u16,
    flags: u16,
    abi_version: u16,
    entry_version: u16,
    entry_offset: u32,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(AGENT_IMAGE_HEADER_BYTES + code.len());
    bytes.extend_from_slice(b"AGNTIMG\0");
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&architecture.to_le_bytes());
    bytes.extend_from_slice(&image_kind.to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&abi_version.to_le_bytes());
    bytes.extend_from_slice(&entry_version.to_le_bytes());
    bytes.extend_from_slice(&entry_offset.to_le_bytes());
    bytes.extend_from_slice(&(code.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(code);
    bytes
}

fn mutated(bytes: &[u8], offset: usize, value: u8) -> Vec<u8> {
    let mut changed = bytes.to_vec();
    changed[offset] = value;
    changed
}

fn mutated_u16(bytes: &[u8], offset: usize, value: u16) -> Vec<u8> {
    let mut changed = bytes.to_vec();
    changed[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    changed
}

fn mutated_u32(bytes: &[u8], offset: usize, value: u32) -> Vec<u8> {
    let mut changed = bytes.to_vec();
    changed[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    changed
}
