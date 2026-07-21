use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageKind, AgentImageRecord, AgentImageStatus,
    ResourceId,
};
use agent_kernel_x86_64::agent_image::{
    sha256_digest, AgentImageCapsule, AgentImageFormat, AgentImageLoadError, VerifiedAgentImage,
    AGENT_PACKAGE_RELOCATION_BYTES, AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES,
    AGENT_PACKAGE_V2_HEADER_BYTES, MAX_AGENT_RELOCATIONS, MAX_AGENT_RODATA_BYTES,
};

const ABI_VERSION: u16 = 1;
const ENTRY_VERSION: u16 = 1;
const CODE_SEGMENT: u16 = 0;
const RODATA_SEGMENT: u16 = 1;

#[test]
fn package_v2_parses_code_rodata_and_abs64_relocation() {
    let code = [0x48, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0x48, 0x8b, 0x00];
    let rodata = *b"PKGV2RODATA-PROOF";
    let bytes = package_v2(&code, &rodata, &[(2, 0)]);
    let parsed = AgentImageCapsule::parse(&bytes).unwrap();

    assert_eq!(AGENT_PACKAGE_V2_HEADER_BYTES, 48);
    assert_eq!(AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES, 24);
    assert_eq!(AGENT_PACKAGE_RELOCATION_BYTES, 24);
    assert_eq!(parsed.format(), AgentImageFormat::PackageV2);
    assert_eq!(parsed.header().format_version(), 2);
    assert_eq!(parsed.entry_offset(), 0);
    assert_eq!(parsed.code(), code);
    assert_eq!(parsed.rodata(), rodata);
    assert_eq!(parsed.code_page_count(), 1);
    assert_eq!(parsed.rodata_page_count(), 1);
    assert_eq!(parsed.relocation_count(), 1);

    let relocation = parsed.relocation(0).unwrap();
    assert_eq!(relocation.target_offset(), 2);
    assert_eq!(relocation.addend(), 0);
    assert_eq!(
        relocation.resolve(0x0000_4000_0001_0000),
        Some(0x0000_4000_0001_0000)
    );
    assert_eq!(parsed.relocation(1), None);
}

#[test]
fn package_v2_digest_binds_tables_code_rodata_and_relocations() {
    let code = [0x48, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0x90];
    let rodata = *b"immutable";
    let bytes = package_v2(&code, &rodata, &[(2, 1)]);
    let digest = sha256_digest(&bytes);
    let verified = VerifiedAgentImage::verify(record(digest), &bytes).unwrap();

    assert_eq!(verified.format(), AgentImageFormat::PackageV2);
    assert_eq!(verified.code(), code);
    assert_eq!(verified.rodata(), rodata);
    assert_eq!(verified.relocation_count(), 1);

    let relocation_offset =
        AGENT_PACKAGE_V2_HEADER_BYTES + 2 * AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES;
    let code_offset = relocation_offset + AGENT_PACKAGE_RELOCATION_BYTES;
    for offset in [relocation_offset + 16, code_offset + 10, bytes.len() - 1] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        assert_eq!(
            VerifiedAgentImage::verify(record(digest), &changed),
            Err(AgentImageLoadError::DigestMismatch)
        );
    }
}

#[test]
fn package_v2_rejects_noncanonical_segment_tables() {
    let valid = package_v2(&[0x90], b"R", &[]);
    let cases = [
        (
            mutated_u16(&valid, 28, 1),
            AgentImageLoadError::InvalidSegmentCount,
        ),
        (
            mutated_u32(&valid, 32, 49),
            AgentImageLoadError::InvalidSegmentTable,
        ),
        (
            mutated_u16(&valid, 48, 2),
            AgentImageLoadError::InvalidSegmentKind,
        ),
        (
            mutated_u16(&valid, 50, 7),
            AgentImageLoadError::InvalidSegmentFlags,
        ),
        (
            mutated_u32(&valid, 52, 8),
            AgentImageLoadError::InvalidSegmentAlignment,
        ),
        (
            mutated_u32(&valid, 64, 2),
            AgentImageLoadError::InvalidSegmentLength,
        ),
        (
            mutated_u32(&valid, 56, 121),
            AgentImageLoadError::InvalidSegmentLayout,
        ),
        (
            mutated_u32(&valid, 44, 1),
            AgentImageLoadError::ReservedNotZero,
        ),
    ];

    for (bytes, expected) in cases {
        assert_eq!(AgentImageCapsule::parse(&bytes), Err(expected));
    }
}

#[test]
fn package_v2_enforces_rodata_and_relocation_bounds() {
    assert_eq!(MAX_AGENT_RODATA_BYTES, 65_536);
    assert_eq!(MAX_AGENT_RELOCATIONS, 64);

    let maximal = package_v2(&[0x90], &vec![0x5a; MAX_AGENT_RODATA_BYTES], &[]);
    let parsed = AgentImageCapsule::parse(&maximal).unwrap();
    assert_eq!(parsed.rodata_page_count(), 16);

    let oversized = package_v2(&[0x90], &vec![0; MAX_AGENT_RODATA_BYTES + 1], &[]);
    assert_eq!(
        AgentImageCapsule::parse(&oversized),
        Err(AgentImageLoadError::InvalidSegmentLength)
    );

    let too_many = package_v2(&[0; 8 * 65], b"R", &vec![(0, 0); 65]);
    assert_eq!(
        AgentImageCapsule::parse(&too_many),
        Err(AgentImageLoadError::InvalidRelocationCount)
    );
}

fn record(digest: AgentImageDigest) -> AgentImageRecord {
    AgentImageRecord {
        id: AgentImageId::new(7),
        owner: AgentId::new(1),
        resource: ResourceId::new(1),
        kind: AgentImageKind::Worker,
        digest,
        abi_version: ABI_VERSION,
        entry_version: ENTRY_VERSION,
        status: AgentImageStatus::Verified,
    }
}

fn package_v2(code: &[u8], rodata: &[u8], relocations: &[(u32, i64)]) -> Vec<u8> {
    let relocation_offset =
        AGENT_PACKAGE_V2_HEADER_BYTES + 2 * AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES;
    let code_offset = relocation_offset + relocations.len() * AGENT_PACKAGE_RELOCATION_BYTES;
    let rodata_offset = code_offset + code.len();
    let package_length = rodata_offset + rodata.len();
    let mut bytes = Vec::with_capacity(package_length);

    bytes.extend_from_slice(b"AGNTIMG\0");
    put_u16(&mut bytes, 2);
    put_u16(&mut bytes, 1);
    put_u16(&mut bytes, 1);
    put_u16(&mut bytes, 0);
    put_u16(&mut bytes, ABI_VERSION);
    put_u16(&mut bytes, ENTRY_VERSION);
    put_u16(&mut bytes, CODE_SEGMENT);
    put_u16(&mut bytes, 0);
    put_u32(&mut bytes, 0);
    put_u16(&mut bytes, 2);
    put_u16(&mut bytes, relocations.len() as u16);
    put_u32(&mut bytes, AGENT_PACKAGE_V2_HEADER_BYTES as u32);
    put_u32(&mut bytes, relocation_offset as u32);
    put_u32(&mut bytes, package_length as u32);
    put_u32(&mut bytes, 0);

    segment(&mut bytes, 1, 5, code_offset, code.len());
    segment(&mut bytes, 2, 1, rodata_offset, rodata.len());
    for (target_offset, addend) in relocations {
        put_u16(&mut bytes, CODE_SEGMENT);
        put_u16(&mut bytes, 1);
        put_u16(&mut bytes, RODATA_SEGMENT);
        put_u16(&mut bytes, 0);
        put_u32(&mut bytes, *target_offset);
        put_u32(&mut bytes, 0);
        bytes.extend_from_slice(&addend.to_le_bytes());
    }
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
    let mut changed = bytes.to_vec();
    changed[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    changed
}

fn mutated_u32(bytes: &[u8], offset: usize, value: u32) -> Vec<u8> {
    let mut changed = bytes.to_vec();
    changed[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    changed
}
