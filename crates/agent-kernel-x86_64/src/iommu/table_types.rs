//! Intel VT-d legacy table identities, limits, and validation errors.
//!
//! These architecture types validate caller-owned frame addresses before the
//! shared-domain page owner can publish any requester or mapping entry.

use agent_kernel_core::DMA_PAGE_BYTES;

pub const VTD_ADDRESS_WIDTH: u8 = 39;
pub const VTD_REQUESTER_CAPACITY: usize = 256;
pub const VTD_MAPPING_CAPACITY: usize = 512;

const ADDRESS_LIMIT: u64 = 1_u64 << VTD_ADDRESS_WIDTH;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VtdTableError {
    AddressMisaligned(u64),
    AddressOutOfRange(u64),
    DuplicateTableAddress,
    UnsupportedPciSegment(u16),
    InvalidIova(u64),
    NoRequesterPresent,
    RequesterAlreadyPresent,
    RequesterNotPresent,
    PciBusMismatch { expected: u8, actual: u8 },
    DomainMismatch,
    MappingAlreadyPresent,
    MappingNotPresent,
    MappingWindowMismatch { expected: u64, actual: u64 },
    TableStateCorrupted,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VtdDomainId(u16);

impl VtdDomainId {
    pub const fn new(raw: u16) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VtdLegacyTableAddresses {
    root: u64,
    context: u64,
    level3: u64,
    level2: u64,
    level1: u64,
}

impl VtdLegacyTableAddresses {
    pub fn new(
        root: u64,
        context: u64,
        level3: u64,
        level2: u64,
        level1: u64,
    ) -> Result<Self, VtdTableError> {
        let values = [root, context, level3, level2, level1];
        for address in values {
            validate_frame(address)?;
        }
        for (index, address) in values.iter().enumerate() {
            if values[..index].contains(address) {
                return Err(VtdTableError::DuplicateTableAddress);
            }
        }
        Ok(Self {
            root,
            context,
            level3,
            level2,
            level1,
        })
    }

    pub const fn root(self) -> u64 {
        self.root
    }

    pub(super) const fn context(self) -> u64 {
        self.context
    }

    pub(super) const fn level3(self) -> u64 {
        self.level3
    }

    pub(super) const fn level2(self) -> u64 {
        self.level2
    }

    pub(super) const fn level1(self) -> u64 {
        self.level1
    }
}

pub(super) fn validate_frame(address: u64) -> Result<(), VtdTableError> {
    if !address.is_multiple_of(DMA_PAGE_BYTES) {
        return Err(VtdTableError::AddressMisaligned(address));
    }
    if address >= ADDRESS_LIMIT {
        return Err(VtdTableError::AddressOutOfRange(address));
    }
    Ok(())
}
