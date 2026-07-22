use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageKind, AgentImageRecord, AgentImageStatus,
    ResourceId,
};
use agent_kernel_x86_64::agent_image::{
    sha256_digest, AgentImageCapsule, AgentImageFormat, AgentImageKindScope, AgentImageLoadError,
    AgentImageSignerId, AgentImageTrust, AgentImageTrustPolicy, TrustedAgentSigner,
    TrustedSignerStatus, VerifiedAgentImage, AGENT_PACKAGE_RELOCATION_BYTES,
    AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES, AGENT_PACKAGE_SIGNATURE_BYTES,
    AGENT_PACKAGE_V3_HEADER_BYTES,
};

const ABI_VERSION: u16 = 1;
const ENTRY_VERSION: u16 = 1;
const WORKER_KIND: u16 = 1;
const SUPERVISOR_KIND: u16 = 4;
const CODE_SEGMENT: u16 = 0;
const RODATA_SEGMENT: u16 = 1;
const SIGNER_DOMAIN: &[u8] = b"AGENT_KERNEL_ED25519_SIGNER_V1\0";

#[test]
fn package_v3_parses_canonical_signed_segments() {
    let code = [0x48, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0x48, 0x8b, 0x00];
    let rodata = *b"PKGV3-SIGNED-RODATA";
    let fixture = signed_package(WORKER_KIND, ABI_VERSION, &code, &rodata, &[(2, 0)], [7; 32]);
    let parsed = AgentImageCapsule::parse(&fixture.bytes).unwrap();

    assert_eq!(AGENT_PACKAGE_V3_HEADER_BYTES, 88);
    assert_eq!(AGENT_PACKAGE_SIGNATURE_BYTES, 64);
    assert_eq!(parsed.format(), AgentImageFormat::SignedPackageV3);
    assert_eq!(parsed.header().format_version(), 3);
    assert_eq!(parsed.code(), code);
    assert_eq!(parsed.rodata(), rodata);
    assert_eq!(parsed.relocation_count(), 1);
    assert_eq!(
        parsed.signer_id(),
        Some(AgentImageSignerId::new(fixture.signer_id))
    );
    assert_eq!(
        parsed.signed_bytes().unwrap().len(),
        fixture.signature_offset
    );
    assert_eq!(parsed.signature().unwrap(), fixture.signature);
    assert_eq!(parsed.raw().len(), fixture.signature_offset + 64);
}

#[test]
fn package_v3_rejects_noncanonical_signature_envelope() {
    let fixture = signed_package(WORKER_KIND, ABI_VERSION, &[0x90], b"R", &[], [8; 32]);
    assert_eq!(
        AgentImageCapsule::parse(&fixture.bytes[..AGENT_PACKAGE_V3_HEADER_BYTES - 1]),
        Err(AgentImageLoadError::HeaderTruncated)
    );
    let valid = fixture.bytes;
    let cases = [
        (
            mutated_u16(&valid, 14, 0),
            AgentImageLoadError::UnsupportedFlags,
        ),
        (
            mutated_u32(&valid, 32, 89),
            AgentImageLoadError::InvalidSegmentTable,
        ),
        (
            mutated_u32(&valid, 36, 137),
            AgentImageLoadError::InvalidRelocationTable,
        ),
        (
            mutated_u16(&valid, 80, 2),
            AgentImageLoadError::UnsupportedSignatureAlgorithm,
        ),
        (
            mutated_u16(&valid, 82, 63),
            AgentImageLoadError::InvalidSignatureLength,
        ),
        (
            mutated_u32(&valid, 40, fixture.signature_offset as u32 - 1),
            AgentImageLoadError::InvalidSignatureLayout,
        ),
        (
            mutated_u32(&valid, 84, 1),
            AgentImageLoadError::ReservedNotZero,
        ),
        (
            mutated_range(&valid, 48, &[0; 32]),
            AgentImageLoadError::InvalidSignerId,
        ),
    ];

    for (bytes, expected) in cases {
        assert_eq!(AgentImageCapsule::parse(&bytes), Err(expected));
    }

    let mut trailing = valid.clone();
    trailing.push(0);
    assert_eq!(
        AgentImageCapsule::parse(&trailing),
        Err(AgentImageLoadError::LengthMismatch)
    );
}

#[test]
fn signed_verification_binds_package_to_one_active_trusted_signer() {
    let fixture = signed_package(WORKER_KIND, ABI_VERSION, &[0x90], b"trusted", &[], [9; 32]);
    let signer = trusted_signer(&fixture, WORKER_KIND, 1, 2, TrustedSignerStatus::Active);
    let policy = AgentImageTrustPolicy::new([signer]);
    let record = record(
        sha256_digest(&fixture.bytes),
        AgentImageKind::Worker,
        ABI_VERSION,
    );
    let verified = VerifiedAgentImage::verify_signed(record, &fixture.bytes, &policy).unwrap();
    let signer_id = AgentImageSignerId::new(fixture.signer_id);

    assert_eq!(verified.format(), AgentImageFormat::SignedPackageV3);
    assert_eq!(verified.signer_id(), Some(signer_id));
    assert_eq!(verified.trust(), AgentImageTrust::Signed(signer_id));
    assert_eq!(
        VerifiedAgentImage::verify(record, &fixture.bytes),
        Err(AgentImageLoadError::SignatureVerificationRequired)
    );
}

#[test]
fn signed_verification_rejects_payload_and_signature_tampering_after_digest_rebind() {
    let fixture = signed_package(
        WORKER_KIND,
        ABI_VERSION,
        &[0x90, 0x90],
        b"trusted",
        &[],
        [10; 32],
    );
    let policy = AgentImageTrustPolicy::new([trusted_signer(
        &fixture,
        WORKER_KIND,
        1,
        1,
        TrustedSignerStatus::Active,
    )]);

    for offset in [fixture.code_offset, fixture.signature_offset + 7] {
        let mut changed = fixture.bytes.clone();
        changed[offset] ^= 1;
        let rebound = record(sha256_digest(&changed), AgentImageKind::Worker, ABI_VERSION);
        assert_eq!(
            VerifiedAgentImage::verify_signed(rebound, &changed, &policy),
            Err(AgentImageLoadError::SignatureInvalid)
        );
    }
}

#[test]
fn trust_policy_rejects_missing_ambiguous_revoked_and_mismatched_keys() {
    let fixture = signed_package(WORKER_KIND, ABI_VERSION, &[0x90], b"trusted", &[], [11; 32]);
    let record = record(
        sha256_digest(&fixture.bytes),
        AgentImageKind::Worker,
        ABI_VERSION,
    );
    let active = trusted_signer(&fixture, WORKER_KIND, 1, 1, TrustedSignerStatus::Active);

    assert_eq!(
        VerifiedAgentImage::verify_signed(
            record,
            &fixture.bytes,
            &AgentImageTrustPolicy::<0>::new([])
        ),
        Err(AgentImageLoadError::SignerNotTrusted)
    );
    assert_eq!(
        VerifiedAgentImage::verify_signed(
            record,
            &fixture.bytes,
            &AgentImageTrustPolicy::new([active, active]),
        ),
        Err(AgentImageLoadError::TrustPolicyAmbiguous)
    );

    let revoked = trusted_signer(&fixture, WORKER_KIND, 1, 1, TrustedSignerStatus::Revoked);
    assert_eq!(
        VerifiedAgentImage::verify_signed(
            record,
            &fixture.bytes,
            &AgentImageTrustPolicy::new([revoked]),
        ),
        Err(AgentImageLoadError::SignerRevoked)
    );

    let other = signed_package(WORKER_KIND, ABI_VERSION, &[0x90], b"other", &[], [12; 32]);
    let mismatched = TrustedAgentSigner::new(
        AgentImageSignerId::new(fixture.signer_id),
        other.public_key,
        AgentImageKindScope::only(WORKER_KIND).unwrap(),
        1,
        1,
        TrustedSignerStatus::Active,
    )
    .unwrap();
    assert_eq!(
        VerifiedAgentImage::verify_signed(
            record,
            &fixture.bytes,
            &AgentImageTrustPolicy::new([mismatched]),
        ),
        Err(AgentImageLoadError::SignerKeyIdMismatch)
    );
}

#[test]
fn trust_policy_enforces_image_kind_and_abi_scope() {
    let fixture = signed_package(WORKER_KIND, ABI_VERSION, &[0x90], b"trusted", &[], [13; 32]);
    let record = record(
        sha256_digest(&fixture.bytes),
        AgentImageKind::Worker,
        ABI_VERSION,
    );

    let wrong_kind = trusted_signer(&fixture, SUPERVISOR_KIND, 1, 1, TrustedSignerStatus::Active);
    assert_eq!(
        VerifiedAgentImage::verify_signed(
            record,
            &fixture.bytes,
            &AgentImageTrustPolicy::new([wrong_kind]),
        ),
        Err(AgentImageLoadError::SignerScopeMismatch)
    );

    let wrong_abi = trusted_signer(&fixture, WORKER_KIND, 2, 3, TrustedSignerStatus::Active);
    assert_eq!(
        VerifiedAgentImage::verify_signed(
            record,
            &fixture.bytes,
            &AgentImageTrustPolicy::new([wrong_abi]),
        ),
        Err(AgentImageLoadError::SignerAbiMismatch)
    );
}

#[test]
fn signed_loader_rejects_digest_mismatch_before_trust_and_unsigned_v2_packages() {
    let fixture = signed_package(WORKER_KIND, ABI_VERSION, &[0x90], b"trusted", &[], [14; 32]);
    let policy = AgentImageTrustPolicy::new([trusted_signer(
        &fixture,
        WORKER_KIND,
        1,
        1,
        TrustedSignerStatus::Active,
    )]);
    let wrong_digest = record(
        AgentImageDigest::new([0; 32]),
        AgentImageKind::Worker,
        ABI_VERSION,
    );
    assert_eq!(
        VerifiedAgentImage::verify_signed(wrong_digest, &fixture.bytes, &policy),
        Err(AgentImageLoadError::DigestMismatch)
    );

    let unsigned = package_v2(&[0x90], b"legacy");
    let unsigned_record = record(
        sha256_digest(&unsigned),
        AgentImageKind::Worker,
        ABI_VERSION,
    );
    assert_eq!(
        VerifiedAgentImage::verify_signed(unsigned_record, &unsigned, &policy),
        Err(AgentImageLoadError::SignatureRequired)
    );
    assert_eq!(
        VerifiedAgentImage::verify(unsigned_record, &unsigned)
            .unwrap()
            .trust(),
        AgentImageTrust::DigestPinned
    );
}

#[derive(Clone)]
struct SignedFixture {
    bytes: Vec<u8>,
    public_key: [u8; 32],
    signer_id: [u8; 32],
    signature: [u8; 64],
    code_offset: usize,
    signature_offset: usize,
}

fn signed_package(
    image_kind: u16,
    abi_version: u16,
    code: &[u8],
    rodata: &[u8],
    relocations: &[(u32, i64)],
    signing_seed: [u8; 32],
) -> SignedFixture {
    let signing_key = SigningKey::from_bytes(&signing_seed);
    let public_key = signing_key.verifying_key().to_bytes();
    let signer_id = signer_id(public_key);
    let relocation_offset =
        AGENT_PACKAGE_V3_HEADER_BYTES + 2 * AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES;
    let code_offset = relocation_offset + relocations.len() * AGENT_PACKAGE_RELOCATION_BYTES;
    let rodata_offset = code_offset + code.len();
    let signature_offset = rodata_offset + rodata.len();
    let package_length = signature_offset + AGENT_PACKAGE_SIGNATURE_BYTES;
    let mut bytes = Vec::with_capacity(package_length);

    bytes.extend_from_slice(b"AGNTIMG\0");
    for value in [3_u16, 1, image_kind, 1, abi_version, ENTRY_VERSION, 0, 0] {
        put_u16(&mut bytes, value);
    }
    put_u32(&mut bytes, 0);
    put_u16(&mut bytes, 2);
    put_u16(&mut bytes, relocations.len() as u16);
    put_u32(&mut bytes, AGENT_PACKAGE_V3_HEADER_BYTES as u32);
    put_u32(&mut bytes, relocation_offset as u32);
    put_u32(&mut bytes, signature_offset as u32);
    put_u32(&mut bytes, package_length as u32);
    bytes.extend_from_slice(&signer_id);
    put_u16(&mut bytes, 1);
    put_u16(&mut bytes, AGENT_PACKAGE_SIGNATURE_BYTES as u16);
    put_u32(&mut bytes, 0);

    segment(&mut bytes, 1, 5, code_offset, code.len());
    segment(&mut bytes, 2, 1, rodata_offset, rodata.len());
    for (target, addend) in relocations {
        for value in [CODE_SEGMENT, 1, RODATA_SEGMENT, 0] {
            put_u16(&mut bytes, value);
        }
        put_u32(&mut bytes, *target);
        put_u32(&mut bytes, 0);
        bytes.extend_from_slice(&addend.to_le_bytes());
    }
    bytes.extend_from_slice(code);
    bytes.extend_from_slice(rodata);
    let signature = signing_key.sign(&bytes).to_bytes();
    bytes.extend_from_slice(&signature);

    SignedFixture {
        bytes,
        public_key,
        signer_id,
        signature,
        code_offset,
        signature_offset,
    }
}

fn trusted_signer(
    fixture: &SignedFixture,
    image_kind: u16,
    minimum_abi: u16,
    maximum_abi: u16,
    status: TrustedSignerStatus,
) -> TrustedAgentSigner {
    TrustedAgentSigner::new(
        AgentImageSignerId::new(fixture.signer_id),
        fixture.public_key,
        AgentImageKindScope::only(image_kind).unwrap(),
        minimum_abi,
        maximum_abi,
        status,
    )
    .unwrap()
}

fn signer_id(public_key: [u8; 32]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(SIGNER_DOMAIN);
    digest.update(public_key);
    digest.finalize().into()
}

fn record(digest: AgentImageDigest, kind: AgentImageKind, abi_version: u16) -> AgentImageRecord {
    AgentImageRecord {
        id: AgentImageId::new(7),
        owner: AgentId::new(1),
        resource: ResourceId::new(1),
        kind,
        digest,
        abi_version,
        entry_version: ENTRY_VERSION,
        status: AgentImageStatus::Verified,
    }
}

fn package_v2(code: &[u8], rodata: &[u8]) -> Vec<u8> {
    let relocation_offset = 48 + 2 * AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES;
    let code_offset = relocation_offset;
    let rodata_offset = code_offset + code.len();
    let package_length = rodata_offset + rodata.len();
    let mut bytes = Vec::with_capacity(package_length);
    bytes.extend_from_slice(b"AGNTIMG\0");
    for value in [2_u16, 1, WORKER_KIND, 0, ABI_VERSION, ENTRY_VERSION, 0, 0] {
        put_u16(&mut bytes, value);
    }
    put_u32(&mut bytes, 0);
    put_u16(&mut bytes, 2);
    put_u16(&mut bytes, 0);
    put_u32(&mut bytes, 48);
    put_u32(&mut bytes, relocation_offset as u32);
    put_u32(&mut bytes, package_length as u32);
    put_u32(&mut bytes, 0);
    segment(&mut bytes, 1, 5, code_offset, code.len());
    segment(&mut bytes, 2, 1, rodata_offset, rodata.len());
    bytes.extend_from_slice(code);
    bytes.extend_from_slice(rodata);
    bytes
}

fn segment(bytes: &mut Vec<u8>, kind: u16, flags: u16, offset: usize, length: usize) {
    put_u16(bytes, kind);
    put_u16(bytes, flags);
    put_u32(bytes, 4096);
    put_u32(bytes, offset as u32);
    put_u32(bytes, length as u32);
    put_u32(bytes, length as u32);
    put_u32(bytes, 0);
}

fn put_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn mutated_u16(bytes: &[u8], offset: usize, value: u16) -> Vec<u8> {
    mutated_range(bytes, offset, &value.to_le_bytes())
}

fn mutated_u32(bytes: &[u8], offset: usize, value: u32) -> Vec<u8> {
    mutated_range(bytes, offset, &value.to_le_bytes())
}

fn mutated_range(bytes: &[u8], offset: usize, value: &[u8]) -> Vec<u8> {
    let mut changed = bytes.to_vec();
    changed[offset..offset + value.len()].copy_from_slice(value);
    changed
}
