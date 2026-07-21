use agent_kernel_x86_64::agent_image::{
    AgentImageCapsule, AgentImageLoadError, AGENT_PACKAGE_RELOCATION_BYTES,
    AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES, AGENT_PACKAGE_V2_HEADER_BYTES,
};

#[test]
fn package_v2_rejects_unsupported_or_out_of_range_relocations() {
    let valid = package(&vec![0; 4104], b"RODATA", &[(8, 0)]);
    let relocation = AGENT_PACKAGE_V2_HEADER_BYTES + 2 * AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES;
    let code_offset = relocation + AGENT_PACKAGE_RELOCATION_BYTES;
    let cases = [
        (
            mutated_u16(&valid, relocation, 1),
            AgentImageLoadError::InvalidRelocationTarget,
        ),
        (
            mutated_u16(&valid, relocation + 2, 2),
            AgentImageLoadError::UnsupportedRelocationKind,
        ),
        (
            mutated_u16(&valid, relocation + 4, 0),
            AgentImageLoadError::InvalidRelocationSymbol,
        ),
        (
            mutated_u16(&valid, relocation + 6, 1),
            AgentImageLoadError::ReservedNotZero,
        ),
        (
            mutated_u32(&valid, relocation + 8, 4090),
            AgentImageLoadError::InvalidRelocationTarget,
        ),
        (
            mutated_u32(&valid, relocation + 12, 1),
            AgentImageLoadError::ReservedNotZero,
        ),
        (
            mutated_i64(&valid, relocation + 16, -1),
            AgentImageLoadError::InvalidRelocationAddend,
        ),
        (
            mutated_i64(&valid, relocation + 16, 6),
            AgentImageLoadError::InvalidRelocationAddend,
        ),
        (
            mutated_byte(&valid, code_offset + 8, 1),
            AgentImageLoadError::RelocationPlaceholderNotZero,
        ),
    ];

    for (bytes, expected) in cases {
        assert_eq!(AgentImageCapsule::parse(&bytes), Err(expected));
    }
}

#[test]
fn package_v2_requires_sorted_nonoverlapping_relocations() {
    let code = vec![0; 64];
    let reversed = package(&code, b"RODATA", &[(24, 0), (8, 0)]);
    assert_eq!(
        AgentImageCapsule::parse(&reversed),
        Err(AgentImageLoadError::RelocationOrderInvalid)
    );

    let overlapping = package(&code, b"RODATA", &[(8, 0), (12, 0)]);
    assert_eq!(
        AgentImageCapsule::parse(&overlapping),
        Err(AgentImageLoadError::RelocationOverlap)
    );
}

fn package(code: &[u8], rodata: &[u8], relocations: &[(u32, i64)]) -> Vec<u8> {
    let relocation_offset =
        AGENT_PACKAGE_V2_HEADER_BYTES + 2 * AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES;
    let code_offset = relocation_offset + relocations.len() * AGENT_PACKAGE_RELOCATION_BYTES;
    let rodata_offset = code_offset + code.len();
    let package_length = rodata_offset + rodata.len();
    let mut bytes = Vec::with_capacity(package_length);
    bytes.extend_from_slice(b"AGNTIMG\0");
    for value in [2_u16, 1, 1, 0, 1, 1, 0, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&(relocations.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&(AGENT_PACKAGE_V2_HEADER_BYTES as u32).to_le_bytes());
    bytes.extend_from_slice(&(relocation_offset as u32).to_le_bytes());
    bytes.extend_from_slice(&(package_length as u32).to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    segment(&mut bytes, 1, 5, code_offset, code.len());
    segment(&mut bytes, 2, 1, rodata_offset, rodata.len());
    for (target, addend) in relocations {
        for value in [0_u16, 1, 1, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&target.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&addend.to_le_bytes());
    }
    bytes.extend_from_slice(code);
    bytes.extend_from_slice(rodata);
    bytes
}

fn segment(bytes: &mut Vec<u8>, kind: u16, flags: u16, offset: usize, length: usize) {
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&4096_u32.to_le_bytes());
    bytes.extend_from_slice(&(offset as u32).to_le_bytes());
    bytes.extend_from_slice(&(length as u32).to_le_bytes());
    bytes.extend_from_slice(&(length as u32).to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
}

fn mutated_byte(bytes: &[u8], offset: usize, value: u8) -> Vec<u8> {
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

fn mutated_i64(bytes: &[u8], offset: usize, value: i64) -> Vec<u8> {
    let mut changed = bytes.to_vec();
    changed[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    changed
}
