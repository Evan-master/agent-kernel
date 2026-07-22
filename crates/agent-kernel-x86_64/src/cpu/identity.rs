//! CPU identity values imported from x86 firmware topology.
//!
//! This architecture module separates stable logical indices from sparse APIC
//! identifiers. Values are fixed-width, allocator-free, and validated before
//! they enter topology or runtime state.

pub const MAX_CPU_COUNT: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CpuIndex(u16);

impl CpuIndex {
    pub const BSP: Self = Self(0);

    pub const fn new(raw: u16) -> Option<Self> {
        if (raw as usize) < MAX_CPU_COUNT {
            Some(Self(raw))
        } else {
            None
        }
    }

    pub const fn get(self) -> u16 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ApicId(u32);

impl ApicId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessorSource {
    LocalApic,
    LocalX2Apic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FirmwareCpuFlags {
    enabled: bool,
    online_capable: bool,
}

impl FirmwareCpuFlags {
    pub const ENABLED_BIT: u32 = 1;
    pub const ONLINE_CAPABLE_BIT: u32 = 1 << 1;

    pub const fn new(enabled: bool, online_capable: bool) -> Self {
        Self {
            enabled,
            online_capable,
        }
    }

    pub const fn from_madt_bits(bits: u32) -> Self {
        Self::new(
            bits & Self::ENABLED_BIT != 0,
            bits & Self::ONLINE_CAPABLE_BIT != 0,
        )
    }

    pub const fn enabled(self) -> bool {
        self.enabled
    }

    pub const fn online_capable(self) -> bool {
        self.online_capable
    }

    pub const fn usable(self) -> bool {
        self.enabled || self.online_capable
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FirmwareProcessor {
    uid: u32,
    apic_id: ApicId,
    source: ProcessorSource,
    flags: FirmwareCpuFlags,
}

impl FirmwareProcessor {
    pub const fn new(
        uid: u32,
        apic_id: ApicId,
        source: ProcessorSource,
        flags: FirmwareCpuFlags,
    ) -> Self {
        Self {
            uid,
            apic_id,
            source,
            flags,
        }
    }

    pub const fn uid(self) -> u32 {
        self.uid
    }

    pub const fn apic_id(self) -> ApicId {
        self.apic_id
    }

    pub const fn source(self) -> ProcessorSource {
        self.source
    }

    pub const fn flags(self) -> FirmwareCpuFlags {
        self.flags
    }
}
