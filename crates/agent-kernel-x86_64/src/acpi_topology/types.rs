//! Fixed-capacity values produced by validated MADT parsing.
//!
//! These types keep firmware addresses, interrupt routing, and CPU topology in
//! immutable architecture-owned state. Public fields remain inaccessible so
//! later APIC code can rely on parser invariants.

use crate::cpu::{CpuTopology, TopologyError, MAX_CPU_COUNT};

pub const MAX_IO_APICS: usize = 8;
pub const MAX_INTERRUPT_OVERRIDES: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptPolarity {
    SameAsBus,
    ActiveHigh,
    ActiveLow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptTrigger {
    SameAsBus,
    Edge,
    Level,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoApicDescriptor {
    id: u8,
    address: u64,
    gsi_base: u32,
}

impl IoApicDescriptor {
    pub(super) const EMPTY: Self = Self::new(0, 0, 0);

    pub(super) const fn new(id: u8, address: u64, gsi_base: u32) -> Self {
        Self {
            id,
            address,
            gsi_base,
        }
    }

    pub const fn id(self) -> u8 {
        self.id
    }

    pub const fn address(self) -> u64 {
        self.address
    }

    pub const fn gsi_base(self) -> u32 {
        self.gsi_base
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptSourceOverride {
    source_irq: u8,
    gsi: u32,
    polarity: InterruptPolarity,
    trigger: InterruptTrigger,
}

impl InterruptSourceOverride {
    pub(super) const EMPTY: Self = Self::new(
        0,
        0,
        InterruptPolarity::SameAsBus,
        InterruptTrigger::SameAsBus,
    );

    pub(super) const fn new(
        source_irq: u8,
        gsi: u32,
        polarity: InterruptPolarity,
        trigger: InterruptTrigger,
    ) -> Self {
        Self {
            source_irq,
            gsi,
            polarity,
            trigger,
        }
    }

    pub const fn source_irq(self) -> u8 {
        self.source_irq
    }

    pub const fn gsi(self) -> u32 {
        self.gsi
    }

    pub const fn polarity(self) -> InterruptPolarity {
        self.polarity
    }

    pub const fn trigger(self) -> InterruptTrigger {
        self.trigger
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcpiMachineTopology<const CPU_CAPACITY: usize = MAX_CPU_COUNT> {
    cpus: CpuTopology<CPU_CAPACITY>,
    local_apic_address: u64,
    supports_legacy_pic: bool,
    io_apics: [IoApicDescriptor; MAX_IO_APICS],
    io_apic_count: usize,
    overrides: [InterruptSourceOverride; MAX_INTERRUPT_OVERRIDES],
    override_count: usize,
}

impl<const CPU_CAPACITY: usize> AcpiMachineTopology<CPU_CAPACITY> {
    pub(super) const fn new(
        cpus: CpuTopology<CPU_CAPACITY>,
        local_apic_address: u64,
        supports_legacy_pic: bool,
        io_apics: [IoApicDescriptor; MAX_IO_APICS],
        io_apic_count: usize,
        overrides: [InterruptSourceOverride; MAX_INTERRUPT_OVERRIDES],
        override_count: usize,
    ) -> Self {
        Self {
            cpus,
            local_apic_address,
            supports_legacy_pic,
            io_apics,
            io_apic_count,
            overrides,
            override_count,
        }
    }

    pub const fn cpus(&self) -> &CpuTopology<CPU_CAPACITY> {
        &self.cpus
    }

    pub const fn local_apic_address(&self) -> u64 {
        self.local_apic_address
    }

    pub const fn supports_legacy_pic(&self) -> bool {
        self.supports_legacy_pic
    }

    pub fn io_apics(&self) -> &[IoApicDescriptor] {
        &self.io_apics[..self.io_apic_count]
    }

    pub fn interrupt_overrides(&self) -> &[InterruptSourceOverride] {
        &self.overrides[..self.override_count]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcpiTopologyError {
    MissingMadt,
    InvalidRsdpSignature,
    InvalidRsdpChecksum,
    InvalidRsdpOemId,
    InvalidRsdpLength(usize),
    RootAddressMissing,
    InvalidRootTableSignature,
    InvalidRootTableChecksum,
    InvalidRootTableLength(usize),
    AcpiTableConstruction,
    TableTooShort,
    InvalidSignature,
    InvalidChecksum,
    LengthOutOfBounds {
        declared: usize,
        available: usize,
    },
    MalformedEntry {
        offset: usize,
        entry_type: u8,
        length: usize,
    },
    EntryOutOfBounds {
        offset: usize,
        length: usize,
        table_length: usize,
    },
    Cpu(TopologyError),
    InvalidLocalApicAddress(u64),
    DuplicateLocalApicAddressOverride,
    MissingIoApic,
    IoApicCapacity,
    InvalidIoApicAddress(u64),
    DuplicateIoApicId(u8),
    DuplicateIoApicAddress(u64),
    DuplicateIoApicGsiBase(u32),
    UnsupportedIoSapic,
    UnsupportedLocalSapic,
    InvalidInterruptBus(u8),
    InvalidInterruptFlags(u16),
    DuplicateSourceIrq(u8),
    InterruptOverrideCapacity,
}

impl From<TopologyError> for AcpiTopologyError {
    fn from(value: TopologyError) -> Self {
        Self::Cpu(value)
    }
}
