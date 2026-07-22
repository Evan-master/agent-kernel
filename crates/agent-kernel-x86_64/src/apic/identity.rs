//! Boot-processor identity from CPUID and IA32_APIC_BASE.
//!
//! Pure decoding lets the bare-metal entry cross-check CPU feature, BSP role,
//! APIC mode, and MMIO base against ACPI before touching APIC registers.

use crate::cpu::ApicId;

use super::LocalApicBase;

const CPUID_APIC_FEATURE: u32 = 1 << 9;
const APIC_ID_SHIFT: u32 = 24;
const APIC_BASE_BSP: u64 = 1 << 8;
const APIC_BASE_X2APIC: u64 = 1 << 10;
const APIC_BASE_ENABLED: u64 = 1 << 11;
const APIC_BASE_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
const APIC_BASE_ALLOWED: u64 =
    APIC_BASE_ADDRESS_MASK | APIC_BASE_BSP | APIC_BASE_X2APIC | APIC_BASE_ENABLED;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuidApicIdentity {
    initial_apic_id: ApicId,
}

impl CpuidApicIdentity {
    pub const fn from_leaf1(ebx: u32, edx: u32) -> Option<Self> {
        if edx & CPUID_APIC_FEATURE == 0 {
            None
        } else {
            Some(Self {
                initial_apic_id: ApicId::new(ebx >> APIC_ID_SHIFT),
            })
        }
    }

    pub const fn initial_apic_id(self) -> ApicId {
        self.initial_apic_id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApicBaseMsr {
    base: LocalApicBase,
    x2apic_enabled: bool,
    boot_processor: bool,
}

impl ApicBaseMsr {
    pub const fn from_raw(raw: u64) -> Option<Self> {
        if raw & !APIC_BASE_ALLOWED != 0 || raw & APIC_BASE_ENABLED == 0 {
            return None;
        }
        let Some(base) = LocalApicBase::new(raw & APIC_BASE_ADDRESS_MASK) else {
            return None;
        };
        Some(Self {
            base,
            x2apic_enabled: raw & APIC_BASE_X2APIC != 0,
            boot_processor: raw & APIC_BASE_BSP != 0,
        })
    }

    pub const fn base(self) -> LocalApicBase {
        self.base
    }

    pub const fn enabled(self) -> bool {
        true
    }

    pub const fn x2apic_enabled(self) -> bool {
        self.x2apic_enabled
    }

    pub const fn boot_processor(self) -> bool {
        self.boot_processor
    }
}
