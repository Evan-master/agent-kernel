//! Typed modern Virtio PCI vendor-capability decoding.
//!
//! The x86 architecture layer validates the common vendor layout, required
//! lengths, BAR selectors, and BAR-relative regions before a Virtio driver can
//! map Common, Notify, ISR, Device, or PCI configuration structures.

use super::{
    config_field, PciBarIndex, PciCapability, PciConfigAccess, PciFunctionAddress,
    PCI_CAPABILITY_ID_VENDOR_SPECIFIC,
};

const BASE_CAPABILITY_BYTES: u8 = 16;
const NOTIFY_CAPABILITY_BYTES: u8 = 20;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioPciCapabilityKind {
    CommonConfiguration,
    Notify,
    Isr,
    DeviceConfiguration,
    PciConfiguration,
}

impl VirtioPciCapabilityKind {
    const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::CommonConfiguration),
            2 => Some(Self::Notify),
            3 => Some(Self::Isr),
            4 => Some(Self::DeviceConfiguration),
            5 => Some(Self::PciConfiguration),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioPciCapability {
    kind: VirtioPciCapabilityKind,
    bar: PciBarIndex,
    id: u8,
    offset: u32,
    length: u32,
    notify_offset_multiplier: Option<u32>,
}

impl VirtioPciCapability {
    pub fn decode<A: PciConfigAccess>(
        access: &mut A,
        address: PciFunctionAddress,
        record: PciCapability,
    ) -> Result<Self, VirtioPciCapabilityError> {
        if record.id() != PCI_CAPABILITY_ID_VENDOR_SPECIFIC {
            return Err(VirtioPciCapabilityError::WrongCapabilityId {
                actual: record.id(),
            });
        }
        let capability_offset = u16::from(record.offset());
        let header = config_field::read_u32(access, address, capability_offset);
        if header as u8 != PCI_CAPABILITY_ID_VENDOR_SPECIFIC {
            return Err(VirtioPciCapabilityError::WrongCapabilityId {
                actual: header as u8,
            });
        }
        let capability_len = (header >> 16) as u8;
        let configuration_type = (header >> 24) as u8;
        let kind = VirtioPciCapabilityKind::from_raw(configuration_type)
            .ok_or(VirtioPciCapabilityError::UnsupportedConfigurationType { configuration_type })?;
        let required = match kind {
            VirtioPciCapabilityKind::Notify | VirtioPciCapabilityKind::PciConfiguration => {
                NOTIFY_CAPABILITY_BYTES
            }
            _ => BASE_CAPABILITY_BYTES,
        };
        if capability_len < required {
            return Err(VirtioPciCapabilityError::CapabilityTooShort {
                kind,
                required,
                actual: capability_len,
            });
        }
        if capability_offset + u16::from(capability_len) > 0x100 {
            return Err(VirtioPciCapabilityError::CapabilityOutsideConfig {
                offset: record.offset(),
                length: capability_len,
            });
        }

        let selector = config_field::read_u32(access, address, capability_offset + 4);
        let bar_raw = selector as u8;
        let bar = PciBarIndex::new(bar_raw)
            .ok_or(VirtioPciCapabilityError::InvalidBar { bar: bar_raw })?;
        let offset = config_field::read_u32(access, address, capability_offset + 8);
        let length = config_field::read_u32(access, address, capability_offset + 12);
        if length == 0 && kind != VirtioPciCapabilityKind::PciConfiguration {
            return Err(VirtioPciCapabilityError::EmptyRegion { kind });
        }
        let notify_offset_multiplier = if kind == VirtioPciCapabilityKind::Notify {
            Some(config_field::read_u32(
                access,
                address,
                capability_offset + 16,
            ))
        } else {
            None
        };
        Ok(Self {
            kind,
            bar,
            id: (selector >> 8) as u8,
            offset,
            length,
            notify_offset_multiplier,
        })
    }

    pub const fn kind(self) -> VirtioPciCapabilityKind {
        self.kind
    }

    pub const fn bar(self) -> PciBarIndex {
        self.bar
    }

    pub const fn id(self) -> u8 {
        self.id
    }

    pub const fn offset(self) -> u32 {
        self.offset
    }

    pub const fn length(self) -> u32 {
        self.length
    }

    pub const fn notify_offset_multiplier(self) -> Option<u32> {
        self.notify_offset_multiplier
    }

    pub fn bar_region(
        self,
        bar: PciBarIndex,
        bar_size: u64,
    ) -> Result<VirtioPciBarRegion, VirtioPciCapabilityError> {
        if self.length == 0 {
            return Err(VirtioPciCapabilityError::EmptyRegion { kind: self.kind });
        }
        if bar != self.bar {
            return Err(VirtioPciCapabilityError::BarMismatch {
                expected: self.bar,
                actual: bar,
            });
        }
        if u64::from(self.offset) + u64::from(self.length) > bar_size {
            return Err(VirtioPciCapabilityError::RegionOutsideBar {
                offset: self.offset,
                length: self.length,
                bar_size,
            });
        }
        Ok(VirtioPciBarRegion {
            bar,
            offset: self.offset,
            length: self.length,
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioPciBarRegion {
    bar: PciBarIndex,
    offset: u32,
    length: u32,
}

impl VirtioPciBarRegion {
    pub const fn new(bar: PciBarIndex, offset: u32, length: u32) -> Option<Self> {
        if length == 0 || offset.checked_add(length).is_none() {
            return None;
        }
        Some(Self {
            bar,
            offset,
            length,
        })
    }

    pub const fn bar(self) -> PciBarIndex {
        self.bar
    }

    pub const fn offset(self) -> u32 {
        self.offset
    }

    pub const fn length(self) -> u32 {
        self.length
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioPciCapabilityError {
    WrongCapabilityId {
        actual: u8,
    },
    UnsupportedConfigurationType {
        configuration_type: u8,
    },
    CapabilityTooShort {
        kind: VirtioPciCapabilityKind,
        required: u8,
        actual: u8,
    },
    CapabilityOutsideConfig {
        offset: u8,
        length: u8,
    },
    InvalidBar {
        bar: u8,
    },
    EmptyRegion {
        kind: VirtioPciCapabilityKind,
    },
    BarMismatch {
        expected: PciBarIndex,
        actual: PciBarIndex,
    },
    RegionOutsideBar {
        offset: u32,
        length: u32,
        bar_size: u64,
    },
}
