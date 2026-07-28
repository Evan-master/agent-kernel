//! Strict, allocator-free ACPI DMAR parsing.
//!
//! This architecture module validates the DMAR header and retains bounded DRHD
//! units and device scopes. It does not access IOMMU registers or interpret
//! operating-system policy.

const DMAR_HEADER_BYTES: usize = 48;
const DRHD_HEADER_BYTES: usize = 16;
const DEVICE_SCOPE_HEADER_BYTES: usize = 6;
pub const MAX_DMAR_PATH_ENTRIES: usize = 4;

mod parser;

pub use parser::parse_dmar;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DmarTableError {
    TableTooShort,
    InvalidSignature,
    InvalidLength { declared: usize, available: usize },
    InvalidChecksum,
    InvalidHostAddressWidth(u8),
    ReservedFieldNonZero,
    InvalidStructureLength { length: usize },
    StructureOutOfBounds { length: usize, remaining: usize },
    HardwareUnitCapacityExceeded { capacity: usize },
    DeviceScopeCapacityExceeded { capacity: usize },
    InvalidHardwareUnitFlags(u8),
    InvalidRegisterBase(u64),
    InvalidDeviceScopeKind(u8),
    InvalidDeviceScopeLength { length: usize },
    DevicePathCapacityExceeded { capacity: usize },
    InvalidDevicePath,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DmarPciRequester {
    segment: u16,
    bus: u8,
    device: u8,
    function: u8,
}

impl DmarPciRequester {
    pub const fn new(segment: u16, bus: u8, device: u8, function: u8) -> Option<Self> {
        if device > 31 || function > 7 {
            return None;
        }
        Some(Self {
            segment,
            bus,
            device,
            function,
        })
    }

    pub const fn segment(self) -> u16 {
        self.segment
    }

    pub const fn bus(self) -> u8 {
        self.bus
    }

    pub const fn device(self) -> u8 {
        self.device
    }

    pub const fn function(self) -> u8 {
        self.function
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DmarDeviceScopeKind {
    PciEndpoint,
    PciBridge,
    IoApic,
    Hpet,
    AcpiNamespace,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DmarPciPath {
    device: u8,
    function: u8,
}

impl DmarPciPath {
    pub const fn device(self) -> u8 {
        self.device
    }

    pub const fn function(self) -> u8 {
        self.function
    }

    const EMPTY: Self = Self {
        device: 0,
        function: 0,
    };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DmarDeviceScope {
    kind: DmarDeviceScopeKind,
    enumeration_id: u8,
    start_bus: u8,
    path: [DmarPciPath; MAX_DMAR_PATH_ENTRIES],
    path_len: usize,
}

impl DmarDeviceScope {
    pub const fn kind(self) -> DmarDeviceScopeKind {
        self.kind
    }

    pub const fn enumeration_id(self) -> u8 {
        self.enumeration_id
    }

    pub const fn start_bus(self) -> u8 {
        self.start_bus
    }

    pub fn path(&self) -> &[DmarPciPath] {
        &self.path[..self.path_len]
    }

    fn matches_endpoint(self, requester: DmarPciRequester) -> bool {
        self.kind == DmarDeviceScopeKind::PciEndpoint
            && self.start_bus == requester.bus
            && self.path_len == 1
            && self.path[0].device == requester.device
            && self.path[0].function == requester.function
    }

    const EMPTY: Self = Self {
        kind: DmarDeviceScopeKind::PciEndpoint,
        enumeration_id: 0,
        start_bus: 0,
        path: [DmarPciPath::EMPTY; MAX_DMAR_PATH_ENTRIES],
        path_len: 0,
    };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DmarHardwareUnit<const SCOPES: usize> {
    include_all: bool,
    segment: u16,
    register_base: u64,
    scopes: [DmarDeviceScope; SCOPES],
    scope_len: usize,
}

impl<const SCOPES: usize> DmarHardwareUnit<SCOPES> {
    pub const fn include_all(&self) -> bool {
        self.include_all
    }

    pub const fn segment(&self) -> u16 {
        self.segment
    }

    pub const fn register_base(&self) -> u64 {
        self.register_base
    }

    pub fn scopes(&self) -> &[DmarDeviceScope] {
        &self.scopes[..self.scope_len]
    }

    fn covers(&self, requester: DmarPciRequester) -> bool {
        self.segment == requester.segment
            && (self.include_all
                || self
                    .scopes()
                    .iter()
                    .any(|scope| scope.matches_endpoint(requester)))
    }

    const fn empty() -> Self {
        Self {
            include_all: false,
            segment: 0,
            register_base: 0,
            scopes: [DmarDeviceScope::EMPTY; SCOPES],
            scope_len: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DmarTable<const UNITS: usize, const SCOPES: usize> {
    host_address_width: u8,
    interrupt_remapping: bool,
    units: [DmarHardwareUnit<SCOPES>; UNITS],
    unit_len: usize,
}

impl<const UNITS: usize, const SCOPES: usize> DmarTable<UNITS, SCOPES> {
    pub const fn host_address_width(&self) -> u8 {
        self.host_address_width
    }

    pub const fn interrupt_remapping(&self) -> bool {
        self.interrupt_remapping
    }

    pub fn hardware_units(&self) -> &[DmarHardwareUnit<SCOPES>] {
        &self.units[..self.unit_len]
    }

    pub fn hardware_unit_for(
        &self,
        requester: DmarPciRequester,
    ) -> Option<DmarHardwareUnit<SCOPES>> {
        self.hardware_units()
            .iter()
            .find(|unit| unit.covers(requester) && !unit.include_all)
            .or_else(|| {
                self.hardware_units()
                    .iter()
                    .find(|unit| unit.covers(requester))
            })
            .copied()
    }
}
