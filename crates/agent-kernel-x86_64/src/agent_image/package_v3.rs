//! Canonical parser for Ed25519-signed Agent Package v3.

use super::{
    format::{read_u16, read_u32, supported_image_kind, AgentImageCapsule, AgentImageHeader},
    segmented::{self, CODE_SEGMENT, SEGMENT_COUNT},
    AgentImageLoadError, AgentImageSignerId, AGENT_IMAGE_ARCH_X86_64, AGENT_IMAGE_MAGIC,
    AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES, AGENT_PACKAGE_SIGNATURE_BYTES,
    AGENT_PACKAGE_V3_HEADER_BYTES, MAX_AGENT_RELOCATIONS,
};

const FORMAT_VERSION: u16 = 3;
const SIGNED_FLAG: u16 = 1;
const ED25519_ALGORITHM: u16 = 1;

#[derive(Copy, Clone)]
struct PackageHeader {
    architecture: u16,
    image_kind: u16,
    flags: u16,
    abi_version: u16,
    entry_version: u16,
    entry_segment: u16,
    reserved_short: u16,
    entry_offset: u32,
    segment_count: u16,
    relocation_count: u16,
    segment_table_offset: u32,
    relocation_table_offset: u32,
    signature_offset: u32,
    package_length: u32,
    signer_id: AgentImageSignerId,
    signature_algorithm: u16,
    signature_length: u16,
    reserved: u32,
}

pub(super) fn parse(bytes: &[u8]) -> Result<AgentImageCapsule<'_>, AgentImageLoadError> {
    if bytes.len() < AGENT_PACKAGE_V3_HEADER_BYTES {
        return Err(AgentImageLoadError::HeaderTruncated);
    }
    if &bytes[..AGENT_IMAGE_MAGIC.len()] != AGENT_IMAGE_MAGIC {
        return Err(AgentImageLoadError::InvalidMagic);
    }

    let package_header = read_header(bytes);
    validate_header(package_header)?;
    if package_header.package_length as usize != bytes.len() {
        return Err(AgentImageLoadError::LengthMismatch);
    }
    let signature_offset = package_header.signature_offset as usize;
    let signed_package_length = signature_offset
        .checked_add(AGENT_PACKAGE_SIGNATURE_BYTES)
        .ok_or(AgentImageLoadError::InvalidSignatureLayout)?;
    if signed_package_length != package_header.package_length as usize {
        return Err(AgentImageLoadError::InvalidSignatureLayout);
    }

    let payload = segmented::parse(
        bytes,
        package_header.segment_table_offset as usize,
        package_header.relocation_table_offset as usize,
        package_header.relocation_count,
        signature_offset,
        package_header.entry_offset,
    )?;
    let signed_bytes = bytes
        .get(..signature_offset)
        .ok_or(AgentImageLoadError::InvalidSignatureLayout)?;
    let signature = bytes
        .get(signature_offset..signed_package_length)
        .ok_or(AgentImageLoadError::InvalidSignatureLayout)?;
    let header = AgentImageHeader::new(
        FORMAT_VERSION,
        package_header.architecture,
        package_header.image_kind,
        package_header.abi_version,
        package_header.entry_version,
        package_header.entry_offset,
        payload.code.len() as u32,
        payload.rodata.len() as u32,
        package_header.relocation_count,
    );
    Ok(AgentImageCapsule::package_v3(
        header,
        bytes,
        payload.code,
        payload.rodata,
        payload.relocations,
        package_header.signer_id,
        signed_bytes,
        signature,
    ))
}

fn read_header(bytes: &[u8]) -> PackageHeader {
    let mut signer_id = [0; 32];
    signer_id.copy_from_slice(&bytes[48..80]);
    PackageHeader {
        architecture: read_u16(bytes, 10),
        image_kind: read_u16(bytes, 12),
        flags: read_u16(bytes, 14),
        abi_version: read_u16(bytes, 16),
        entry_version: read_u16(bytes, 18),
        entry_segment: read_u16(bytes, 20),
        reserved_short: read_u16(bytes, 22),
        entry_offset: read_u32(bytes, 24),
        segment_count: read_u16(bytes, 28),
        relocation_count: read_u16(bytes, 30),
        segment_table_offset: read_u32(bytes, 32),
        relocation_table_offset: read_u32(bytes, 36),
        signature_offset: read_u32(bytes, 40),
        package_length: read_u32(bytes, 44),
        signer_id: AgentImageSignerId::new(signer_id),
        signature_algorithm: read_u16(bytes, 80),
        signature_length: read_u16(bytes, 82),
        reserved: read_u32(bytes, 84),
    }
}

fn validate_header(header: PackageHeader) -> Result<(), AgentImageLoadError> {
    if header.architecture != AGENT_IMAGE_ARCH_X86_64 {
        return Err(AgentImageLoadError::UnsupportedArchitecture);
    }
    if !supported_image_kind(header.image_kind) {
        return Err(AgentImageLoadError::UnsupportedImageKind);
    }
    if header.flags != SIGNED_FLAG {
        return Err(AgentImageLoadError::UnsupportedFlags);
    }
    if header.abi_version == 0 || header.entry_version == 0 {
        return Err(AgentImageLoadError::InvalidVersion);
    }
    if header.entry_segment != CODE_SEGMENT {
        return Err(AgentImageLoadError::EntryOutOfRange);
    }
    if header.reserved_short != 0 || header.reserved != 0 {
        return Err(AgentImageLoadError::ReservedNotZero);
    }
    if header.segment_count != SEGMENT_COUNT {
        return Err(AgentImageLoadError::InvalidSegmentCount);
    }
    if usize::from(header.relocation_count) > MAX_AGENT_RELOCATIONS {
        return Err(AgentImageLoadError::InvalidRelocationCount);
    }
    if header.segment_table_offset as usize != AGENT_PACKAGE_V3_HEADER_BYTES {
        return Err(AgentImageLoadError::InvalidSegmentTable);
    }
    let expected_relocation_offset = AGENT_PACKAGE_V3_HEADER_BYTES
        + usize::from(header.segment_count) * AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES;
    if header.relocation_table_offset as usize != expected_relocation_offset {
        return Err(AgentImageLoadError::InvalidRelocationTable);
    }
    if header.signature_algorithm != ED25519_ALGORITHM {
        return Err(AgentImageLoadError::UnsupportedSignatureAlgorithm);
    }
    if header.signature_length as usize != AGENT_PACKAGE_SIGNATURE_BYTES {
        return Err(AgentImageLoadError::InvalidSignatureLength);
    }
    if header.signer_id.is_zero() {
        return Err(AgentImageLoadError::InvalidSignerId);
    }
    Ok(())
}
