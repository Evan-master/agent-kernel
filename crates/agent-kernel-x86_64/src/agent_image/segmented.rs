//! Shared structural validation for segmented Agent Packages.

use super::{
    format::{read_u16, read_u32},
    AgentImageLoadError, AgentImageRelocation, AGENT_PACKAGE_RELOCATION_BYTES,
    AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES, MAX_AGENT_CODE_BYTES, MAX_AGENT_RODATA_BYTES,
};

pub(super) const SEGMENT_COUNT: u16 = 2;
pub(super) const CODE_SEGMENT: u16 = 0;
pub(super) const RODATA_SEGMENT: u16 = 1;

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

pub(super) struct SegmentedPayload<'a> {
    pub(super) code: &'a [u8],
    pub(super) rodata: &'a [u8],
    pub(super) relocations: &'a [u8],
}

pub(super) fn parse(
    bytes: &[u8],
    segment_table_offset: usize,
    relocation_table_offset: usize,
    relocation_count: u16,
    payload_end: usize,
    entry_offset: u32,
) -> Result<SegmentedPayload<'_>, AgentImageLoadError> {
    let code = read_segment(bytes, segment_table_offset)?;
    let rodata = read_segment(
        bytes,
        segment_table_offset + AGENT_PACKAGE_SEGMENT_DESCRIPTOR_BYTES,
    )?;
    validate_segment(
        code,
        CODE_KIND,
        READ_FLAG | EXECUTE_FLAG,
        MAX_AGENT_CODE_BYTES,
    )?;
    validate_segment(rodata, RODATA_KIND, READ_FLAG, MAX_AGENT_RODATA_BYTES)?;

    let relocation_bytes = usize::from(relocation_count)
        .checked_mul(AGENT_PACKAGE_RELOCATION_BYTES)
        .ok_or(AgentImageLoadError::InvalidRelocationTable)?;
    let code_offset = relocation_table_offset
        .checked_add(relocation_bytes)
        .ok_or(AgentImageLoadError::InvalidRelocationTable)?;
    let code_end = code_offset
        .checked_add(code.file_length as usize)
        .ok_or(AgentImageLoadError::InvalidSegmentLayout)?;
    let rodata_end = code_end
        .checked_add(rodata.file_length as usize)
        .ok_or(AgentImageLoadError::InvalidSegmentLayout)?;
    if code.file_offset as usize != code_offset
        || rodata.file_offset as usize != code_end
        || rodata_end != payload_end
    {
        return Err(AgentImageLoadError::InvalidSegmentLayout);
    }
    if entry_offset >= code.file_length {
        return Err(AgentImageLoadError::EntryOutOfRange);
    }

    let relocations = bytes
        .get(relocation_table_offset..code_offset)
        .ok_or(AgentImageLoadError::InvalidRelocationTable)?;
    let code_bytes = bytes
        .get(code_offset..code_end)
        .ok_or(AgentImageLoadError::InvalidSegmentLayout)?;
    let rodata_bytes = bytes
        .get(code_end..rodata_end)
        .ok_or(AgentImageLoadError::InvalidSegmentLayout)?;
    validate_relocations(relocations, code_bytes, rodata.file_length as usize)?;

    Ok(SegmentedPayload {
        code: code_bytes,
        rodata: rodata_bytes,
        relocations,
    })
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
