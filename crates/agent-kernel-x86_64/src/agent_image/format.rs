//! Structural parser and normalized view for Agent image formats.

use super::{
    package_v2, AgentImageLoadError, AgentImageRelocation, AGENT_IMAGE_ARCH_X86_64,
    AGENT_IMAGE_FORMAT_VERSION, AGENT_IMAGE_HEADER_BYTES, AGENT_IMAGE_KIND_FAULT_HANDLER,
    AGENT_IMAGE_KIND_SUPERVISOR, AGENT_IMAGE_KIND_VERIFIER, AGENT_IMAGE_KIND_WORKER,
    AGENT_IMAGE_MAGIC, AGENT_PACKAGE_RELOCATION_BYTES, MAX_AGENT_CODE_BYTES,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageFormat {
    CapsuleV1,
    PackageV2,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageHeader {
    format_version: u16,
    architecture: u16,
    image_kind: u16,
    abi_version: u16,
    entry_version: u16,
    entry_offset: u32,
    code_length: u32,
    rodata_length: u32,
    relocation_count: u16,
}

impl AgentImageHeader {
    pub const fn format_version(self) -> u16 {
        self.format_version
    }

    pub const fn architecture(self) -> u16 {
        self.architecture
    }

    pub const fn image_kind(self) -> u16 {
        self.image_kind
    }

    pub const fn abi_version(self) -> u16 {
        self.abi_version
    }

    pub const fn entry_version(self) -> u16 {
        self.entry_version
    }

    pub const fn entry_offset(self) -> u32 {
        self.entry_offset
    }

    pub const fn code_length(self) -> u32 {
        self.code_length
    }

    pub const fn rodata_length(self) -> u32 {
        self.rodata_length
    }

    pub const fn relocation_count(self) -> u16 {
        self.relocation_count
    }

    pub(super) const fn new(
        format_version: u16,
        architecture: u16,
        image_kind: u16,
        abi_version: u16,
        entry_version: u16,
        entry_offset: u32,
        code_length: u32,
        rodata_length: u32,
        relocation_count: u16,
    ) -> Self {
        Self {
            format_version,
            architecture,
            image_kind,
            abi_version,
            entry_version,
            entry_offset,
            code_length,
            rodata_length,
            relocation_count,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageCapsule<'a> {
    format: AgentImageFormat,
    header: AgentImageHeader,
    raw: &'a [u8],
    code: &'a [u8],
    rodata: &'a [u8],
    relocations: &'a [u8],
}

impl<'a> AgentImageCapsule<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, AgentImageLoadError> {
        if bytes.len() < AGENT_IMAGE_HEADER_BYTES {
            return Err(AgentImageLoadError::HeaderTruncated);
        }
        if &bytes[..AGENT_IMAGE_MAGIC.len()] != AGENT_IMAGE_MAGIC {
            return Err(AgentImageLoadError::InvalidMagic);
        }

        let format_version = read_u16(bytes, 8);
        if format_version == 2 {
            return package_v2::parse(bytes);
        }
        let architecture = read_u16(bytes, 10);
        let image_kind = read_u16(bytes, 12);
        let flags = read_u16(bytes, 14);
        let abi_version = read_u16(bytes, 16);
        let entry_version = read_u16(bytes, 18);
        let entry_offset = read_u32(bytes, 20);
        let code_length = read_u32(bytes, 24);
        let reserved = read_u32(bytes, 28);

        if format_version != AGENT_IMAGE_FORMAT_VERSION {
            return Err(AgentImageLoadError::UnsupportedFormatVersion);
        }
        if architecture != AGENT_IMAGE_ARCH_X86_64 {
            return Err(AgentImageLoadError::UnsupportedArchitecture);
        }
        if !matches!(
            image_kind,
            AGENT_IMAGE_KIND_WORKER
                | AGENT_IMAGE_KIND_VERIFIER
                | AGENT_IMAGE_KIND_FAULT_HANDLER
                | AGENT_IMAGE_KIND_SUPERVISOR
        ) {
            return Err(AgentImageLoadError::UnsupportedImageKind);
        }
        if flags != 0 {
            return Err(AgentImageLoadError::UnsupportedFlags);
        }
        if abi_version == 0 || entry_version == 0 {
            return Err(AgentImageLoadError::InvalidVersion);
        }
        if reserved != 0 {
            return Err(AgentImageLoadError::ReservedNotZero);
        }
        let code_length_usize = code_length as usize;
        if code_length_usize == 0 || code_length_usize > MAX_AGENT_CODE_BYTES {
            return Err(AgentImageLoadError::InvalidCodeLength);
        }
        let expected_length = AGENT_IMAGE_HEADER_BYTES
            .checked_add(code_length_usize)
            .ok_or(AgentImageLoadError::LengthMismatch)?;
        if bytes.len() != expected_length {
            return Err(AgentImageLoadError::LengthMismatch);
        }
        if entry_offset >= code_length {
            return Err(AgentImageLoadError::EntryOutOfRange);
        }

        let header = AgentImageHeader {
            format_version,
            architecture,
            image_kind,
            abi_version,
            entry_version,
            entry_offset,
            code_length,
            rodata_length: 0,
            relocation_count: 0,
        };
        Ok(Self {
            format: AgentImageFormat::CapsuleV1,
            header,
            raw: bytes,
            code: &bytes[AGENT_IMAGE_HEADER_BYTES..],
            rodata: &[],
            relocations: &[],
        })
    }

    pub(super) const fn package_v2(
        header: AgentImageHeader,
        raw: &'a [u8],
        code: &'a [u8],
        rodata: &'a [u8],
        relocations: &'a [u8],
    ) -> Self {
        Self {
            format: AgentImageFormat::PackageV2,
            header,
            raw,
            code,
            rodata,
            relocations,
        }
    }

    pub const fn format(&self) -> AgentImageFormat {
        self.format
    }

    pub const fn header(&self) -> AgentImageHeader {
        self.header
    }

    pub const fn raw(&self) -> &'a [u8] {
        self.raw
    }

    pub const fn code(&self) -> &'a [u8] {
        self.code
    }

    pub const fn rodata(&self) -> &'a [u8] {
        self.rodata
    }

    pub const fn entry_offset(&self) -> u32 {
        self.header.entry_offset
    }

    pub const fn code_page_count(&self) -> usize {
        self.code
            .len()
            .div_ceil(crate::user_memory::PAGE_BYTES as usize)
    }

    pub const fn rodata_page_count(&self) -> usize {
        self.rodata
            .len()
            .div_ceil(crate::user_memory::PAGE_BYTES as usize)
    }

    pub const fn relocation_count(&self) -> usize {
        self.relocations.len() / AGENT_PACKAGE_RELOCATION_BYTES
    }

    pub fn relocation(&self, index: usize) -> Option<AgentImageRelocation> {
        let start = index.checked_mul(AGENT_PACKAGE_RELOCATION_BYTES)?;
        let end = start.checked_add(AGENT_PACKAGE_RELOCATION_BYTES)?;
        AgentImageRelocation::parse(self.relocations.get(start..end)?)
    }
}

pub(super) fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

pub(super) fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

pub(super) const fn supported_image_kind(image_kind: u16) -> bool {
    matches!(
        image_kind,
        AGENT_IMAGE_KIND_WORKER
            | AGENT_IMAGE_KIND_VERIFIER
            | AGENT_IMAGE_KIND_FAULT_HANDLER
            | AGENT_IMAGE_KIND_SUPERVISOR
    )
}
