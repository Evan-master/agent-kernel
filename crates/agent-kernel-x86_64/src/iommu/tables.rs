//! Legacy Intel VT-d translation-table encoding.
//!
//! This module binds one PCI requester and one 4 KiB mapping to caller-owned,
//! page-aligned table frames. It supports the V27 39-bit, three-level proof and
//! contains no allocation or MMIO.

use agent_kernel_core::{DmaAccess, DMA_PAGE_BYTES};

use crate::acpi_topology::DmarPciRequester;

pub const VTD_ADDRESS_WIDTH: u8 = 39;
const ENTRY_COUNT: usize = 512;
const ADDRESS_LIMIT: u64 = 1_u64 << VTD_ADDRESS_WIDTH;
const ENTRY_READ: u64 = 1;
const ENTRY_WRITE: u64 = 1 << 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VtdTableError {
    AddressMisaligned(u64),
    AddressOutOfRange(u64),
    DuplicateTableAddress,
    UnsupportedPciSegment(u16),
    InvalidIova(u64),
    MappingAlreadyPresent,
    MappingNotPresent,
    MappingCapacityExceeded,
    RequesterMismatch,
    DomainMismatch,
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
}

pub struct VtdLegacyTablePages<'a> {
    root: &'a mut [u64; ENTRY_COUNT],
    context: &'a mut [u64; ENTRY_COUNT],
    level3: &'a mut [u64; ENTRY_COUNT],
    level2: &'a mut [u64; ENTRY_COUNT],
    level1: &'a mut [u64; ENTRY_COUNT],
    addresses: VtdLegacyTableAddresses,
    installed_iova: Option<u64>,
    bound_requester: Option<DmarPciRequester>,
    bound_domain: Option<VtdDomainId>,
}

impl<'a> VtdLegacyTablePages<'a> {
    pub fn new(
        root: &'a mut [u64; ENTRY_COUNT],
        context: &'a mut [u64; ENTRY_COUNT],
        level3: &'a mut [u64; ENTRY_COUNT],
        level2: &'a mut [u64; ENTRY_COUNT],
        level1: &'a mut [u64; ENTRY_COUNT],
        addresses: VtdLegacyTableAddresses,
    ) -> Self {
        root.fill(0);
        context.fill(0);
        level3.fill(0);
        level2.fill(0);
        level1.fill(0);
        Self {
            root,
            context,
            level3,
            level2,
            level1,
            addresses,
            installed_iova: None,
            bound_requester: None,
            bound_domain: None,
        }
    }

    pub const fn root_address(&self) -> u64 {
        self.addresses.root
    }

    pub fn root_entries(&self) -> &[u64; ENTRY_COUNT] {
        self.root
    }

    pub fn context_entries(&self) -> &[u64; ENTRY_COUNT] {
        self.context
    }

    pub fn level3_entries(&self) -> &[u64; ENTRY_COUNT] {
        self.level3
    }

    pub fn level2_entries(&self) -> &[u64; ENTRY_COUNT] {
        self.level2
    }

    pub fn level1_entries(&self) -> &[u64; ENTRY_COUNT] {
        self.level1
    }

    pub fn install(
        &mut self,
        requester: DmarPciRequester,
        domain: VtdDomainId,
        iova: u64,
        physical_frame: u64,
        access: DmaAccess,
    ) -> Result<(), VtdTableError> {
        if requester.segment() != 0 {
            return Err(VtdTableError::UnsupportedPciSegment(requester.segment()));
        }
        validate_iova(iova)?;
        validate_frame(physical_frame)?;
        if self.installed_iova == Some(iova) {
            return Err(VtdTableError::MappingAlreadyPresent);
        }
        if self.installed_iova.is_some() {
            return Err(VtdTableError::MappingCapacityExceeded);
        }
        if self.bound_requester.is_some_and(|bound| bound != requester) {
            return Err(VtdTableError::RequesterMismatch);
        }
        if self.bound_domain.is_some_and(|bound| bound != domain) {
            return Err(VtdTableError::DomainMismatch);
        }

        let root_index = usize::from(requester.bus()) * 2;
        let devfn = usize::from((requester.device() << 3) | requester.function());
        let context_index = devfn * 2;
        let level3_index = index_for(iova, 30);
        let level2_index = index_for(iova, 21);
        let level1_index = index_for(iova, 12);

        self.root[root_index] = self.addresses.context | ENTRY_READ;
        self.root[root_index + 1] = 0;
        self.context[context_index] = self.addresses.level3 | ENTRY_READ;
        self.context[context_index + 1] = (u64::from(domain.raw()) << 8) | 1;
        self.level3[level3_index] = self.addresses.level2 | ENTRY_READ | ENTRY_WRITE;
        self.level2[level2_index] = self.addresses.level1 | ENTRY_READ | ENTRY_WRITE;
        self.level1[level1_index] = physical_frame | access_bits(access);
        self.bound_requester = Some(requester);
        self.bound_domain = Some(domain);
        self.installed_iova = Some(iova);
        Ok(())
    }

    pub fn remove(&mut self, iova: u64) -> Result<(), VtdTableError> {
        validate_iova(iova)?;
        if self.installed_iova != Some(iova) {
            return Err(VtdTableError::MappingNotPresent);
        }
        self.level1[index_for(iova, 12)] = 0;
        self.installed_iova = None;
        Ok(())
    }
}

fn validate_frame(address: u64) -> Result<(), VtdTableError> {
    if !address.is_multiple_of(DMA_PAGE_BYTES) {
        return Err(VtdTableError::AddressMisaligned(address));
    }
    if address >= ADDRESS_LIMIT {
        return Err(VtdTableError::AddressOutOfRange(address));
    }
    Ok(())
}

fn validate_iova(iova: u64) -> Result<(), VtdTableError> {
    if !iova.is_multiple_of(DMA_PAGE_BYTES) || iova >= ADDRESS_LIMIT {
        Err(VtdTableError::InvalidIova(iova))
    } else {
        Ok(())
    }
}

const fn index_for(address: u64, shift: u32) -> usize {
    ((address >> shift) & 0x1ff) as usize
}

const fn access_bits(access: DmaAccess) -> u64 {
    match access {
        DmaAccess::Read => ENTRY_READ,
        DmaAccess::Write => ENTRY_WRITE,
        DmaAccess::ReadWrite => ENTRY_READ | ENTRY_WRITE,
    }
}
