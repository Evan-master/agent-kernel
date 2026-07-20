//! Structural parser for the fixed-width Agent Image Capsule V0 header.

use super::{
    AgentImageLoadError, AGENT_IMAGE_ARCH_X86_64, AGENT_IMAGE_FORMAT_VERSION,
    AGENT_IMAGE_HEADER_BYTES, AGENT_IMAGE_KIND_FAULT_HANDLER, AGENT_IMAGE_KIND_SUPERVISOR,
    AGENT_IMAGE_KIND_VERIFIER, AGENT_IMAGE_KIND_WORKER, AGENT_IMAGE_MAGIC, MAX_AGENT_CODE_BYTES,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageHeader {
    format_version: u16,
    architecture: u16,
    image_kind: u16,
    abi_version: u16,
    entry_version: u16,
    entry_offset: u32,
    code_length: u32,
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageCapsule<'a> {
    header: AgentImageHeader,
    raw: &'a [u8],
    code: &'a [u8],
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
        };
        Ok(Self {
            header,
            raw: bytes,
            code: &bytes[AGENT_IMAGE_HEADER_BYTES..],
        })
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

    pub const fn entry_offset(&self) -> u32 {
        self.header.entry_offset
    }

    pub const fn code_page_count(&self) -> usize {
        self.code
            .len()
            .div_ceil(crate::user_memory::PAGE_BYTES as usize)
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
