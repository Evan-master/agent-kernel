//! Canonical parser for segmented Agent Package v2.
//!
//! The parser accepts two packed immutable segments and a bounded ABS64 table.
//! It performs all structural validation without allocation so physical image
//! preparation can trust segment slices and relocation records by construction.

use super::{
    format::{read_u16, read_u32, supported_image_kind, AgentImageCapsule, AgentImageHeader},
    AgentImageLoadError, AgentImageRelocation, AGENT_IMAGE_ARCH_X86_64, AGENT_IMAGE_MAGIC,
    AGENT_PACKAGE_RELOCATION_BYTES, AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES,
    AGENT_PACKAGE_V2_HEADER_BYTES, MAX_AGENT_CODE_BYTES, MAX_AGENT_RELOCATIONS,
    MAX_AGENT_RODATA_BYTES,
};

const FORMAT_VERSION: u16 = 2;
const SEGMENT_COUNT: u16 = 2;
const CODE_SEGMENT: u16 = 0;
const RODATA_SEGMENT: u16 = 1;
const CODE_KIND: u16 = 1;
const RODATA_KIND: u16 = 2;
const READ_FLAG: u16 = 1;
const EXECUTE_FLAG: u16 = 4;
const ABS64_RELOCATION: u16 = 1;
const PAGE_ALIGNMENT: u32 = 4096;

#[derive(Copy, Clone)]
struct Segment {
    kind: u16,
    flags: u16,
    alignment: u32,
    file_offset: u32,
    file_length: u32,
    memory_length: u32,
    reserved: u32,
}

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
    package_length: u32,
    reserved: u32,
}

pub(super) fn parse(bytes: &[u8]) -> Result<AgentImageCapsule<'_>, AgentImageLoadError> {
    if bytes.len() < AGENT_PACKAGE_V2_HEADER_BYTES {
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

    let code = read_segment(bytes, AGENT_PACKAGE_V2_HEADER_BYTES)?;
    let rodata = read_segment(
        bytes,
        AGENT_PACKAGE_V2_HEADER_BYTES + AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES,
    )?;
    validate_segment(
        code,
        CODE_KIND,
        READ_FLAG | EXECUTE_FLAG,
        MAX_AGENT_CODE_BYTES,
    )?;
    validate_segment(rodata, RODATA_KIND, READ_FLAG, MAX_AGENT_RODATA_BYTES)?;

    let relocation_bytes = usize::from(package_header.relocation_count)
        .checked_mul(AGENT_PACKAGE_RELOCATION_BYTES)
        .ok_or(AgentImageLoadError::InvalidRelocationTable)?;
    let payload_offset = (package_header.relocation_table_offset as usize)
        .checked_add(relocation_bytes)
        .ok_or(AgentImageLoadError::InvalidRelocationTable)?;
    let code_end = payload_offset
        .checked_add(code.file_length as usize)
        .ok_or(AgentImageLoadError::InvalidSegmentLayout)?;
    let rodata_end = code_end
        .checked_add(rodata.file_length as usize)
        .ok_or(AgentImageLoadError::InvalidSegmentLayout)?;
    if code.file_offset as usize != payload_offset
        || rodata.file_offset as usize != code_end
        || rodata_end != bytes.len()
    {
        return Err(AgentImageLoadError::InvalidSegmentLayout);
    }
    if package_header.entry_offset >= code.file_length {
        return Err(AgentImageLoadError::EntryOutOfRange);
    }

    let relocations = &bytes[package_header.relocation_table_offset as usize..payload_offset];
    let code_bytes = &bytes[payload_offset..code_end];
    validate_relocations(relocations, code_bytes, rodata.file_length as usize)?;
    let rodata_bytes = &bytes[code_end..rodata_end];
    let header = AgentImageHeader::new(
        FORMAT_VERSION,
        package_header.architecture,
        package_header.image_kind,
        package_header.abi_version,
        package_header.entry_version,
        package_header.entry_offset,
        code.file_length,
        rodata.file_length,
        package_header.relocation_count,
    );
    Ok(AgentImageCapsule::package_v2(
        header,
        bytes,
        code_bytes,
        rodata_bytes,
        relocations,
    ))
}

fn read_header(bytes: &[u8]) -> PackageHeader {
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
        package_length: read_u32(bytes, 40),
        reserved: read_u32(bytes, 44),
    }
}

fn validate_header(header: PackageHeader) -> Result<(), AgentImageLoadError> {
    if header.architecture != AGENT_IMAGE_ARCH_X86_64 {
        return Err(AgentImageLoadError::UnsupportedArchitecture);
    }
    if !supported_image_kind(header.image_kind) {
        return Err(AgentImageLoadError::UnsupportedImageKind);
    }
    if header.flags != 0 {
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
    if header.segment_table_offset as usize != AGENT_PACKAGE_V2_HEADER_BYTES {
        return Err(AgentImageLoadError::InvalidSegmentTable);
    }
    let expected_relocation_offset = AGENT_PACKAGE_V2_HEADER_BYTES
        + usize::from(header.segment_count) * AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES;
    if header.relocation_table_offset as usize != expected_relocation_offset {
        return Err(AgentImageLoadError::InvalidRelocationTable);
    }
    Ok(())
}

fn read_segment(bytes: &[u8], offset: usize) -> Result<Segment, AgentImageLoadError> {
    let end = offset
        .checked_add(AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES)
        .ok_or(AgentImageLoadError::InvalidSegmentTable)?;
    let segment = bytes
        .get(offset..end)
        .ok_or(AgentImageLoadError::InvalidSegmentTable)?;
    Ok(Segment {
        kind: read_u16(segment, 0),
        flags: read_u16(segment, 2),
        alignment: read_u32(segment, 4),
        file_offset: read_u32(segment, 8),
        file_length: read_u32(segment, 12),
        memory_length: read_u32(segment, 16),
        reserved: read_u32(segment, 20),
    })
}

fn validate_segment(
    segment: Segment,
    kind: u16,
    flags: u16,
    maximum: usize,
) -> Result<(), AgentImageLoadError> {
    if segment.kind != kind {
        return Err(AgentImageLoadError::InvalidSegmentKind);
    }
    if segment.flags != flags {
        return Err(AgentImageLoadError::InvalidSegmentFlags);
    }
    if segment.alignment != PAGE_ALIGNMENT {
        return Err(AgentImageLoadError::InvalidSegmentAlignment);
    }
    if segment.file_length == 0
        || segment.file_length as usize > maximum
        || segment.memory_length != segment.file_length
    {
        return Err(AgentImageLoadError::InvalidSegmentLength);
    }
    if segment.reserved != 0 {
        return Err(AgentImageLoadError::ReservedNotZero);
    }
    Ok(())
}

fn validate_relocations(
    bytes: &[u8],
    code: &[u8],
    rodata_length: usize,
) -> Result<(), AgentImageLoadError> {
    let mut previous: Option<AgentImageRelocation> = None;
    let (records, remainder) = bytes.as_chunks::<AGENT_PACKAGE_RELOCATION_BYTES>();
    if !remainder.is_empty() {
        return Err(AgentImageLoadError::InvalidRelocationTable);
    }
    for raw in records {
        let relocation =
            AgentImageRelocation::parse(raw).ok_or(AgentImageLoadError::InvalidRelocationTable)?;
        if relocation.target_segment() != CODE_SEGMENT {
            return Err(AgentImageLoadError::InvalidRelocationTarget);
        }
        if relocation.kind() != ABS64_RELOCATION {
            return Err(AgentImageLoadError::UnsupportedRelocationKind);
        }
        if relocation.symbol_segment() != RODATA_SEGMENT {
            return Err(AgentImageLoadError::InvalidRelocationSymbol);
        }
        if read_u16(raw, 6) != 0 || read_u32(raw, 12) != 0 {
            return Err(AgentImageLoadError::ReservedNotZero);
        }
        let target = relocation.target_offset() as usize;
        let end = target
            .checked_add(8)
            .ok_or(AgentImageLoadError::InvalidRelocationTarget)?;
        if end > code.len()
            || target / PAGE_ALIGNMENT as usize != (end - 1) / PAGE_ALIGNMENT as usize
        {
            return Err(AgentImageLoadError::InvalidRelocationTarget);
        }
        if relocation.addend() < 0 || relocation.addend() as usize >= rodata_length {
            return Err(AgentImageLoadError::InvalidRelocationAddend);
        }
        if let Some(prior) = previous {
            let prior_target = prior.target_offset() as usize;
            if target < prior_target {
                return Err(AgentImageLoadError::RelocationOrderInvalid);
            }
            if target < prior_target + 8 {
                return Err(AgentImageLoadError::RelocationOverlap);
            }
        }
        if code[target..end].iter().any(|byte| *byte != 0) {
            return Err(AgentImageLoadError::RelocationPlaceholderNotZero);
        }
        previous = Some(relocation);
    }
    Ok(())
}
