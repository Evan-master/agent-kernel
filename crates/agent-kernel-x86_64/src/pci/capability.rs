//! Bounded conventional PCI capability-list discovery.
//!
//! The x86 architecture layer validates list ownership, pointer alignment,
//! cycles, and capacity before exposing immutable typed capability records.

use super::{PciConfigAccess, PciConfigRegister, PciFunctionAddress};

pub const PCI_CAPABILITY_ID_MSI: u8 = 0x05;
pub const PCI_CAPABILITY_ID_VENDOR_SPECIFIC: u8 = 0x09;
pub const PCI_CAPABILITY_ID_MSIX: u8 = 0x11;
pub const PCI_CAPABILITY_CAPACITY: usize = 48;

const CAPABILITY_LIST_STATUS: u32 = 1 << 20;
const FIRST_POINTER_OFFSET: u16 = 0x34;
const MINIMUM_POINTER: u8 = 0x40;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciCapability {
    id: u8,
    offset: u8,
}

impl PciCapability {
    pub const fn new(id: u8, offset: u8) -> Option<Self> {
        if valid_pointer(offset) {
            Some(Self { id, offset })
        } else {
            None
        }
    }

    pub const fn id(self) -> u8 {
        self.id
    }

    pub const fn offset(self) -> u8 {
        self.offset
    }

    const EMPTY: Self = Self { id: 0, offset: 0 };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciCapabilityList<const CAPACITY: usize = PCI_CAPABILITY_CAPACITY> {
    records: [PciCapability; CAPACITY],
    len: usize,
}

impl<const CAPACITY: usize> PciCapabilityList<CAPACITY> {
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn all(&self) -> &[PciCapability] {
        &self.records[..self.len]
    }

    pub fn find(&self, id: u8) -> Option<PciCapability> {
        self.all().iter().copied().find(|record| record.id == id)
    }

    const fn new() -> Self {
        Self {
            records: [PciCapability::EMPTY; CAPACITY],
            len: 0,
        }
    }

    fn push(&mut self, record: PciCapability) {
        self.records[self.len] = record;
        self.len += 1;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciCapabilityError {
    ListUnavailable,
    MissingFirstPointer,
    InvalidPointer { pointer: u8 },
    InvalidCapabilityId { offset: u8 },
    CycleDetected { offset: u8 },
    CapacityExceeded { capacity: usize },
}

pub fn discover_pci_capabilities<A: PciConfigAccess>(
    access: &mut A,
    address: PciFunctionAddress,
) -> Result<PciCapabilityList, PciCapabilityError> {
    discover_pci_capabilities_bounded(access, address)
}

pub fn discover_pci_capabilities_bounded<const CAPACITY: usize, A: PciConfigAccess>(
    access: &mut A,
    address: PciFunctionAddress,
) -> Result<PciCapabilityList<CAPACITY>, PciCapabilityError> {
    let command_status = read_register(access, address, 0x04);
    if command_status & CAPABILITY_LIST_STATUS == 0 {
        return Err(PciCapabilityError::ListUnavailable);
    }

    let mut pointer = read_register(access, address, FIRST_POINTER_OFFSET) as u8;
    if pointer == 0 {
        return Err(PciCapabilityError::MissingFirstPointer);
    }

    let mut records = PciCapabilityList::new();
    let mut seen = [false; 64];
    while pointer != 0 {
        if !valid_pointer(pointer) {
            return Err(PciCapabilityError::InvalidPointer { pointer });
        }
        let seen_index = usize::from(pointer) / 4;
        if seen[seen_index] {
            return Err(PciCapabilityError::CycleDetected { offset: pointer });
        }
        if records.len == CAPACITY {
            return Err(PciCapabilityError::CapacityExceeded { capacity: CAPACITY });
        }
        seen[seen_index] = true;

        let header = read_register(access, address, u16::from(pointer));
        let id = header as u8;
        if id == u8::MAX {
            return Err(PciCapabilityError::InvalidCapabilityId { offset: pointer });
        }
        records.push(PciCapability {
            id,
            offset: pointer,
        });
        pointer = (header >> 8) as u8;
    }
    Ok(records)
}

const fn valid_pointer(pointer: u8) -> bool {
    pointer >= MINIMUM_POINTER && pointer & 3 == 0
}

fn read_register<A: PciConfigAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    offset: u16,
) -> u32 {
    let register = PciConfigRegister::new(offset).expect("aligned conventional PCI offset");
    access.read_u32(address, register)
}
