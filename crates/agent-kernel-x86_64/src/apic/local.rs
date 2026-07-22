//! Local APIC MMIO base and register offsets.
//!
//! Physical base validation is independent of a concrete direct-map offset.
//! Register offsets are the aligned xAPIC MMIO locations consumed by the
//! volatile bare-metal backend.

const MMIO_PAGE_MASK: u64 = 4095;
const PHYSICAL_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_ffff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct LocalApicBase(u64);

impl LocalApicBase {
    pub const fn new(physical: u64) -> Option<Self> {
        if physical == 0 || physical & MMIO_PAGE_MASK != 0 || physical & !PHYSICAL_ADDRESS_MASK != 0
        {
            None
        } else {
            Some(Self(physical))
        }
    }

    pub const fn physical(self) -> u64 {
        self.0
    }

    pub const fn virtual_address(self, physical_offset: u64) -> Option<u64> {
        match physical_offset.checked_add(self.0) {
            Some(address) if canonical(address) => Some(address),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum LocalApicRegister {
    Id = 0x20,
    Version = 0x30,
    TaskPriority = 0x80,
    EndOfInterrupt = 0xb0,
    Spurious = 0xf0,
    ErrorStatus = 0x280,
    InterruptCommandLow = 0x300,
    InterruptCommandHigh = 0x310,
    LvtTimer = 0x320,
    LvtLint0 = 0x350,
    LvtLint1 = 0x360,
    LvtError = 0x370,
    TimerInitialCount = 0x380,
    TimerCurrentCount = 0x390,
    TimerDivide = 0x3e0,
}

impl LocalApicRegister {
    pub const fn offset(self) -> u16 {
        self as u16
    }
}

const fn canonical(address: u64) -> bool {
    let upper = address >> 48;
    let sign = (address >> 47) & 1;
    (sign == 0 && upper == 0) || (sign == 1 && upper == 0xffff)
}
