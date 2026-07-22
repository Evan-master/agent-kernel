//! Local APIC interrupt and startup-vector values.
//!
//! Interrupt vectors exclude architectural exception slots. Startup vectors
//! encode one nonzero 4 KiB trampoline page wholly below the 1 MiB real-mode
//! boundary.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ApicVector(u8);

impl ApicVector {
    pub const fn new(raw: u8) -> Option<Self> {
        if raw >= 32 {
            Some(Self(raw))
        } else {
            None
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

pub const APIC_TIMER_VECTOR: ApicVector = ApicVector(0xe0);
pub const APIC_RESCHEDULE_VECTOR: ApicVector = ApicVector(0xe1);
pub const APIC_TLB_SHOOTDOWN_VECTOR: ApicVector = ApicVector(0xe2);
pub const APIC_STARTUP_ERROR_VECTOR: ApicVector = ApicVector(0xe3);
pub const APIC_SPURIOUS_VECTOR: ApicVector = ApicVector(0xff);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct StartupVector(u8);

impl StartupVector {
    const PAGE_BYTES: u64 = 4096;
    const REAL_MODE_LIMIT: u64 = 0x10_0000;

    pub const fn from_trampoline_address(address: u64) -> Option<Self> {
        if address == 0 || address >= Self::REAL_MODE_LIMIT || address & (Self::PAGE_BYTES - 1) != 0
        {
            None
        } else {
            Some(Self((address / Self::PAGE_BYTES) as u8))
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub const fn address(self) -> u64 {
        self.0 as u64 * Self::PAGE_BYTES
    }
}
