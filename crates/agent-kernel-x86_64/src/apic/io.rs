//! Canonical I/O APIC version and redirection-table values.
//!
//! Redirection entries use fixed delivery and physical destination mode. The
//! type exposes only polarity, trigger, mask, vector, and destination fields,
//! leaving reserved and delivery-mode bits zero.

use super::ApicVector;

const VECTOR_MASK: u64 = 0xff;
const DESTINATION_MODE: u64 = 1 << 11;
const POLARITY_LOW: u64 = 1 << 13;
const TRIGGER_LEVEL: u64 = 1 << 15;
const MASKED: u64 = 1 << 16;
const DESTINATION_MASK: u64 = 0xff << 56;
const CANONICAL_MASK: u64 = VECTOR_MASK | POLARITY_LOW | TRIGGER_LEVEL | MASKED | DESTINATION_MASK;
const REDIRECTION_BASE_REGISTER: u16 = 0x10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoApicPolarity {
    ActiveHigh,
    ActiveLow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoApicTrigger {
    Edge,
    Level,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoApicRedirectionEntry {
    raw: u64,
}

impl IoApicRedirectionEntry {
    pub const fn fixed(
        vector: ApicVector,
        destination: u8,
        polarity: IoApicPolarity,
        trigger: IoApicTrigger,
        masked: bool,
    ) -> Self {
        let mut raw = vector.get() as u64 | ((destination as u64) << 56);
        if matches!(polarity, IoApicPolarity::ActiveLow) {
            raw |= POLARITY_LOW;
        }
        if matches!(trigger, IoApicTrigger::Level) {
            raw |= TRIGGER_LEVEL;
        }
        if masked {
            raw |= MASKED;
        }
        Self { raw }
    }

    pub const fn from_raw(raw: u64) -> Option<Self> {
        if raw & !CANONICAL_MASK != 0 || raw & DESTINATION_MODE != 0 {
            return None;
        }
        let vector = raw as u8;
        if vector < 32 {
            return None;
        }
        Some(Self { raw })
    }

    pub const fn raw(self) -> u64 {
        self.raw
    }

    pub const fn low(self) -> u32 {
        self.raw as u32
    }

    pub const fn high(self) -> u32 {
        (self.raw >> 32) as u32
    }

    pub const fn vector(self) -> ApicVector {
        // Construction and decoding reject exception vectors.
        ApicVector::new(self.raw as u8).unwrap()
    }

    pub const fn destination(self) -> u8 {
        (self.raw >> 56) as u8
    }

    pub const fn polarity(self) -> IoApicPolarity {
        if self.raw & POLARITY_LOW == 0 {
            IoApicPolarity::ActiveHigh
        } else {
            IoApicPolarity::ActiveLow
        }
    }

    pub const fn trigger(self) -> IoApicTrigger {
        if self.raw & TRIGGER_LEVEL == 0 {
            IoApicTrigger::Edge
        } else {
            IoApicTrigger::Level
        }
    }

    pub const fn masked(self) -> bool {
        self.raw & MASKED != 0
    }

    pub const fn with_masked(mut self, masked: bool) -> Self {
        if masked {
            self.raw |= MASKED;
        } else {
            self.raw &= !MASKED;
        }
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoApicVersion {
    version: u8,
    maximum_redirection_index: u8,
}

impl IoApicVersion {
    pub const fn from_raw(raw: u32) -> Self {
        Self {
            version: raw as u8,
            maximum_redirection_index: (raw >> 16) as u8,
        }
    }

    pub const fn version(self) -> u8 {
        self.version
    }

    pub const fn redirection_count(self) -> u16 {
        self.maximum_redirection_index as u16 + 1
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoApicRedirectionIndex {
    low_register: u8,
}

impl IoApicRedirectionIndex {
    pub const fn new(index: u8, version: IoApicVersion) -> Option<Self> {
        if index as u16 >= version.redirection_count() {
            return None;
        }
        let register = REDIRECTION_BASE_REGISTER + index as u16 * 2;
        if register + 1 > u8::MAX as u16 {
            None
        } else {
            Some(Self {
                low_register: register as u8,
            })
        }
    }

    pub const fn low_register(self) -> u8 {
        self.low_register
    }

    pub const fn high_register(self) -> u8 {
        self.low_register + 1
    }
}
