//! Legacy Intel VT-d shared-domain translation-table encoding.
//!
//! This x86 architecture owner binds one segment-zero PCI bus and one Domain
//! to caller-owned pages. It independently manages all 256 requester contexts
//! and all 512 4 KiB leaves inside one live 2 MiB IOVA window without allocation
//! or MMIO. The controller owner performs invalidation after each mutation.

use core::sync::atomic::{compiler_fence, Ordering};

use agent_kernel_core::{DmaAccess, DMA_PAGE_BYTES};

use crate::acpi_topology::DmarPciRequester;

use super::table_types::{
    validate_frame, VtdDomainId, VtdLegacyTableAddresses, VtdTableError, VTD_ADDRESS_WIDTH,
};

const ENTRY_COUNT: usize = 512;
const ADDRESS_LIMIT: u64 = 1_u64 << VTD_ADDRESS_WIDTH;
const IOVA_WINDOW_BYTES: u64 = 2 * 1024 * 1024;
const ENTRY_READ: u64 = 1;
const ENTRY_WRITE: u64 = 1 << 1;

pub struct VtdLegacyTablePages<'a> {
    root: &'a mut [u64; ENTRY_COUNT],
    context: &'a mut [u64; ENTRY_COUNT],
    level3: &'a mut [u64; ENTRY_COUNT],
    level2: &'a mut [u64; ENTRY_COUNT],
    level1: &'a mut [u64; ENTRY_COUNT],
    addresses: VtdLegacyTableAddresses,
    bound_bus: Option<u8>,
    bound_domain: Option<VtdDomainId>,
    mapping_window: Option<u64>,
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
            bound_bus: None,
            bound_domain: None,
            mapping_window: None,
        }
    }

    pub const fn root_address(&self) -> u64 {
        self.addresses.root()
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

    pub fn active_requester_count(&self) -> usize {
        self.context
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|entry| entry[0] & ENTRY_READ != 0)
            .count()
    }

    pub fn active_mapping_count(&self) -> usize {
        self.level1
            .iter()
            .filter(|entry| **entry & (ENTRY_READ | ENTRY_WRITE) != 0)
            .count()
    }

    pub fn attach_requester(
        &mut self,
        requester: DmarPciRequester,
        domain: VtdDomainId,
    ) -> Result<(), VtdTableError> {
        self.validate_binding(requester, domain)?;
        let (low_index, high_index) = context_indexes(requester);
        if self.context[low_index] != 0 || self.context[high_index] != 0 {
            if self.requester_is_present(requester, domain)? {
                return Err(VtdTableError::RequesterAlreadyPresent);
            }
            return Err(VtdTableError::TableStateCorrupted);
        }

        let root_index = usize::from(requester.bus()) * 2;
        let expected_root = self.addresses.context() | ENTRY_READ;
        if self.root[root_index] != 0 && self.root[root_index] != expected_root {
            return Err(VtdTableError::TableStateCorrupted);
        }
        if self.root[root_index + 1] != 0 {
            return Err(VtdTableError::TableStateCorrupted);
        }

        self.bound_bus.get_or_insert(requester.bus());
        self.bound_domain.get_or_insert(domain);
        self.root[root_index + 1] = 0;
        compiler_fence(Ordering::Release);
        self.root[root_index] = expected_root;
        self.context[high_index] = context_high(domain);
        compiler_fence(Ordering::Release);
        self.context[low_index] = self.addresses.level3() | ENTRY_READ;
        compiler_fence(Ordering::SeqCst);
        Ok(())
    }

    pub fn detach_requester(
        &mut self,
        requester: DmarPciRequester,
        domain: VtdDomainId,
    ) -> Result<(), VtdTableError> {
        self.validate_binding(requester, domain)?;
        if !self.requester_is_present(requester, domain)? {
            return Err(VtdTableError::RequesterNotPresent);
        }
        let (low_index, high_index) = context_indexes(requester);
        self.context[low_index] = 0;
        compiler_fence(Ordering::Release);
        self.context[high_index] = 0;
        compiler_fence(Ordering::SeqCst);
        Ok(())
    }

    pub fn install_mapping(
        &mut self,
        iova: u64,
        physical_frame: u64,
        access: DmaAccess,
    ) -> Result<(), VtdTableError> {
        validate_iova(iova)?;
        validate_frame(physical_frame)?;
        if self.active_requester_count() == 0 {
            return Err(VtdTableError::NoRequesterPresent);
        }
        let window = iova_window(iova);
        if let Some(expected) = self.mapping_window {
            if expected != window {
                return Err(VtdTableError::MappingWindowMismatch {
                    expected,
                    actual: window,
                });
            }
        }
        let leaf_index = index_for(iova, 12);
        if self.level1[leaf_index] != 0 {
            return Err(VtdTableError::MappingAlreadyPresent);
        }

        let level3_index = index_for(iova, 30);
        let level2_index = index_for(iova, 21);
        let expected_level3 = self.addresses.level2() | ENTRY_READ | ENTRY_WRITE;
        let expected_level2 = self.addresses.level1() | ENTRY_READ | ENTRY_WRITE;
        self.validate_mapping_path(level3_index, level2_index, expected_level3, expected_level2)?;

        self.level1[leaf_index] = physical_frame | access_bits(access);
        if self.mapping_window.is_none() {
            compiler_fence(Ordering::Release);
            self.level2[level2_index] = expected_level2;
            compiler_fence(Ordering::Release);
            self.level3[level3_index] = expected_level3;
            self.mapping_window = Some(window);
        }
        compiler_fence(Ordering::SeqCst);
        Ok(())
    }

    pub fn remove_mapping(&mut self, iova: u64) -> Result<(), VtdTableError> {
        validate_iova(iova)?;
        let Some(window) = self.mapping_window else {
            return Err(VtdTableError::MappingNotPresent);
        };
        if iova_window(iova) != window {
            return Err(VtdTableError::MappingNotPresent);
        }
        let leaf_index = index_for(iova, 12);
        if self.level1[leaf_index] == 0 {
            return Err(VtdTableError::MappingNotPresent);
        }
        let level3_index = index_for(iova, 30);
        let level2_index = index_for(iova, 21);
        let expected_level3 = self.addresses.level2() | ENTRY_READ | ENTRY_WRITE;
        let expected_level2 = self.addresses.level1() | ENTRY_READ | ENTRY_WRITE;
        self.validate_mapping_path(level3_index, level2_index, expected_level3, expected_level2)?;

        self.level1[leaf_index] = 0;
        compiler_fence(Ordering::Release);
        if self.active_mapping_count() == 0 {
            self.level3[level3_index] = 0;
            compiler_fence(Ordering::Release);
            self.level2[level2_index] = 0;
            self.mapping_window = None;
        }
        compiler_fence(Ordering::SeqCst);
        Ok(())
    }

    pub fn install(
        &mut self,
        requester: DmarPciRequester,
        domain: VtdDomainId,
        iova: u64,
        physical_frame: u64,
        access: DmaAccess,
    ) -> Result<(), VtdTableError> {
        let already_present = self.requester_is_present(requester, domain)?;
        if !already_present {
            self.attach_requester(requester, domain)?;
        }
        if let Err(error) = self.install_mapping(iova, physical_frame, access) {
            if !already_present {
                self.detach_requester(requester, domain)?;
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn remove(&mut self, iova: u64) -> Result<(), VtdTableError> {
        self.remove_mapping(iova)
    }

    fn validate_binding(
        &self,
        requester: DmarPciRequester,
        domain: VtdDomainId,
    ) -> Result<(), VtdTableError> {
        if requester.segment() != 0 {
            return Err(VtdTableError::UnsupportedPciSegment(requester.segment()));
        }
        if let Some(expected) = self.bound_bus {
            if expected != requester.bus() {
                return Err(VtdTableError::PciBusMismatch {
                    expected,
                    actual: requester.bus(),
                });
            }
        }
        if self.bound_domain.is_some_and(|expected| expected != domain) {
            return Err(VtdTableError::DomainMismatch);
        }
        Ok(())
    }

    fn requester_is_present(
        &self,
        requester: DmarPciRequester,
        domain: VtdDomainId,
    ) -> Result<bool, VtdTableError> {
        self.validate_binding(requester, domain)?;
        let (low_index, high_index) = context_indexes(requester);
        let low = self.context[low_index];
        let high = self.context[high_index];
        if low == 0 && high == 0 {
            return Ok(false);
        }
        if low != (self.addresses.level3() | ENTRY_READ) || high != context_high(domain) {
            return Err(VtdTableError::TableStateCorrupted);
        }
        Ok(true)
    }

    fn validate_mapping_path(
        &self,
        level3_index: usize,
        level2_index: usize,
        expected_level3: u64,
        expected_level2: u64,
    ) -> Result<(), VtdTableError> {
        let expected = if self.mapping_window.is_some() {
            (expected_level3, expected_level2)
        } else {
            (0, 0)
        };
        if (self.level3[level3_index], self.level2[level2_index]) != expected {
            return Err(VtdTableError::TableStateCorrupted);
        }
        Ok(())
    }
}

fn validate_iova(iova: u64) -> Result<(), VtdTableError> {
    if !iova.is_multiple_of(DMA_PAGE_BYTES) || iova >= ADDRESS_LIMIT {
        Err(VtdTableError::InvalidIova(iova))
    } else {
        Ok(())
    }
}

const fn context_indexes(requester: DmarPciRequester) -> (usize, usize) {
    let devfn = ((requester.device() << 3) | requester.function()) as usize;
    (devfn * 2, devfn * 2 + 1)
}

const fn context_high(domain: VtdDomainId) -> u64 {
    (domain.raw() as u64) << 8 | 1
}

const fn iova_window(iova: u64) -> u64 {
    iova & !(IOVA_WINDOW_BYTES - 1)
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
