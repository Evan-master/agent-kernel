//! Typed PCI coordinates, common-header fields, and device-class values.
//!
//! Constructors close all Configuration Mechanism 1 coordinate and alignment
//! bounds before native I/O can occur.

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PciFunctionAddress {
    bus: u8,
    device: u8,
    function: u8,
}

impl PciFunctionAddress {
    pub const fn new(bus: u8, device: u8, function: u8) -> Option<Self> {
        if device > 31 || function > 7 {
            return None;
        }
        Some(Self {
            bus,
            device,
            function,
        })
    }

    pub const fn coordinates(self) -> (u8, u8, u8) {
        (self.bus, self.device, self.function)
    }

    pub(super) const fn selector_bits(self) -> u32 {
        ((self.bus as u32) << 16) | ((self.device as u32) << 11) | ((self.function as u32) << 8)
    }

    pub(super) const fn from_scan(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device,
            function,
        }
    }

    const ZERO: Self = Self {
        bus: 0,
        device: 0,
        function: 0,
    };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciConfigRegister(u8);

impl PciConfigRegister {
    pub const fn new(offset: u16) -> Option<Self> {
        if offset > 0xfc || offset & 3 != 0 {
            return None;
        }
        Some(Self(offset as u8))
    }

    pub const fn offset(self) -> u8 {
        self.0
    }

    pub(super) const IDENTITY: Self = Self(0x00);
    pub(super) const COMMAND_STATUS: Self = Self(0x04);
    pub(super) const CLASS_REVISION: Self = Self(0x08);
    pub(super) const HEADER: Self = Self(0x0c);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciClassCode {
    base: u8,
    subclass: u8,
    programming_interface: u8,
}

impl PciClassCode {
    pub const fn new(base: u8, subclass: u8, programming_interface: u8) -> Self {
        Self {
            base,
            subclass,
            programming_interface,
        }
    }

    pub const fn base(self) -> u8 {
        self.base
    }

    pub const fn subclass(self) -> u8 {
        self.subclass
    }

    pub const fn programming_interface(self) -> u8 {
        self.programming_interface
    }

    pub const fn is_network_controller(self) -> bool {
        self.base == 0x02
    }

    pub const fn is_display_controller(self) -> bool {
        self.base == 0x03
    }

    pub const fn is_usb_controller(self) -> bool {
        self.base == 0x0c && self.subclass == 0x03
    }

    const EMPTY: Self = Self::new(0, 0, 0);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciFunction {
    address: PciFunctionAddress,
    vendor_id: u16,
    device_id: u16,
    command: u16,
    status: u16,
    revision_id: u8,
    class: PciClassCode,
    header_type: u8,
    multifunction: bool,
}

impl PciFunction {
    pub const fn address(self) -> PciFunctionAddress {
        self.address
    }

    pub const fn vendor_id(self) -> u16 {
        self.vendor_id
    }

    pub const fn device_id(self) -> u16 {
        self.device_id
    }

    pub const fn command(self) -> u16 {
        self.command
    }

    pub const fn status(self) -> u16 {
        self.status
    }

    pub const fn revision_id(self) -> u8 {
        self.revision_id
    }

    pub const fn class(self) -> PciClassCode {
        self.class
    }

    pub const fn header_type(self) -> u8 {
        self.header_type
    }

    pub const fn multifunction(self) -> bool {
        self.multifunction
    }

    pub(super) const fn from_common_header(
        address: PciFunctionAddress,
        identity: u32,
        command_status: u32,
        class_revision: u32,
        header: u32,
    ) -> Self {
        let raw_header = ((header >> 16) & 0xff) as u8;
        Self {
            address,
            vendor_id: identity as u16,
            device_id: (identity >> 16) as u16,
            command: command_status as u16,
            status: (command_status >> 16) as u16,
            revision_id: class_revision as u8,
            class: PciClassCode::new(
                (class_revision >> 24) as u8,
                (class_revision >> 16) as u8,
                (class_revision >> 8) as u8,
            ),
            header_type: raw_header & 0x7f,
            multifunction: raw_header & 0x80 != 0,
        }
    }

    pub(super) const EMPTY: Self = Self {
        address: PciFunctionAddress::ZERO,
        vendor_id: 0,
        device_id: 0,
        command: 0,
        status: 0,
        revision_id: 0,
        class: PciClassCode::EMPTY,
        header_type: 0,
        multifunction: false,
    };
}
