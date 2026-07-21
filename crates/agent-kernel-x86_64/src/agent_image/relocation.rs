//! Bounded Package v2 relocation record.
//!
//! This architecture-library module exposes the immutable relocation facts
//! consumed by Agent-memory initialization. Package parsing validates target,
//! symbol, ordering, and addend policy before any record reaches the loader.

use super::format::{read_u16, read_u32};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageRelocation {
    target_segment: u16,
    kind: u16,
    symbol_segment: u16,
    target_offset: u32,
    addend: i64,
}

impl AgentImageRelocation {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == super::AGENT_PACKAGE_RELOCATION_BYTES).then(|| Self {
            target_segment: read_u16(bytes, 0),
            kind: read_u16(bytes, 2),
            symbol_segment: read_u16(bytes, 4),
            target_offset: read_u32(bytes, 8),
            addend: i64::from_le_bytes([
                bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22],
                bytes[23],
            ]),
        })
    }

    pub(super) const fn target_segment(self) -> u16 {
        self.target_segment
    }

    pub(super) const fn kind(self) -> u16 {
        self.kind
    }

    pub(super) const fn symbol_segment(self) -> u16 {
        self.symbol_segment
    }

    pub const fn target_offset(self) -> u32 {
        self.target_offset
    }

    pub const fn addend(self) -> i64 {
        self.addend
    }

    pub const fn resolve(self, segment_base: u64) -> Option<u64> {
        if self.addend < 0 {
            None
        } else {
            segment_base.checked_add(self.addend as u64)
        }
    }
}
