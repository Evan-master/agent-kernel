//! xAPIC Interrupt Command Register encoding.
//!
//! Commands use physical destination mode and explicit APIC identifiers. The
//! xAPIC profile rejects identifiers wider than eight bits so AP startup and
//! shootdown IPIs cannot silently target another processor.

use crate::cpu::ApicId;

use super::{ApicVector, StartupVector};

const DELIVERY_MODE_SHIFT: u32 = 8;
const DELIVERY_STATUS_PENDING: u32 = 1 << 12;
const LEVEL_ASSERT: u32 = 1 << 14;
const TRIGGER_LEVEL: u32 = 1 << 15;
const INIT_DELIVERY: u32 = 0b101 << DELIVERY_MODE_SHIFT;
const STARTUP_DELIVERY: u32 = 0b110 << DELIVERY_MODE_SHIFT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IcrError {
    DestinationRequiresX2Apic(ApicId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IcrCommand {
    high: u32,
    low: u32,
}

impl IcrCommand {
    pub fn fixed(destination: ApicId, vector: ApicVector) -> Result<Self, IcrError> {
        Self::new(destination, vector.get() as u32)
    }

    pub fn init_assert(destination: ApicId) -> Result<Self, IcrError> {
        Self::new(destination, INIT_DELIVERY | LEVEL_ASSERT | TRIGGER_LEVEL)
    }

    pub fn init_deassert(destination: ApicId) -> Result<Self, IcrError> {
        Self::new(destination, INIT_DELIVERY | TRIGGER_LEVEL)
    }

    pub fn startup(destination: ApicId, vector: StartupVector) -> Result<Self, IcrError> {
        Self::new(destination, STARTUP_DELIVERY | vector.get() as u32)
    }

    pub const fn high(self) -> u32 {
        self.high
    }

    pub const fn low(self) -> u32 {
        self.low
    }

    pub const fn delivery_pending(raw_low: u32) -> bool {
        raw_low & DELIVERY_STATUS_PENDING != 0
    }

    fn new(destination: ApicId, low: u32) -> Result<Self, IcrError> {
        if destination.get() > u8::MAX as u32 {
            return Err(IcrError::DestinationRequiresX2Apic(destination));
        }
        Ok(Self {
            high: destination.get() << 24,
            low,
        })
    }
}
