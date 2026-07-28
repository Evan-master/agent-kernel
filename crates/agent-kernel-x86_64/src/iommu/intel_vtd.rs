//! Bounded Intel VT-d register owner.
//!
//! This module validates the legacy 39-bit capability, programs a root table,
//! performs register-based cache invalidation, controls translation, and
//! decodes one primary fault record.

use core::ptr;

pub const DMAR_VER_REG: u16 = 0x00;
pub const DMAR_CAP_REG: u16 = 0x08;
pub const DMAR_ECAP_REG: u16 = 0x10;
pub const DMAR_GCMD_REG: u16 = 0x18;
pub const DMAR_GSTS_REG: u16 = 0x1c;
pub const DMAR_RTADDR_REG: u16 = 0x20;
pub const DMAR_CCMD_REG: u16 = 0x28;
pub const DMAR_FSTS_REG: u16 = 0x34;
pub const DMAR_FRCD_LOW_REG: u16 = 0x220;
pub const DMAR_FRCD_HIGH_REG: u16 = 0x228;
pub const DMAR_IOTLB_REG: u16 = 0xf8;

const GCMD_TE: u32 = 1 << 31;
const GCMD_SRTP: u32 = 1 << 30;
const GSTS_TES: u32 = 1 << 31;
const GSTS_RTPS: u32 = 1 << 30;
const CCMD_ICC: u64 = 1 << 63;
const CCMD_GLOBAL: u64 = 1 << 61;
const CCMD_ACTUAL_MASK: u64 = 3 << 59;
const CCMD_ACTUAL_GLOBAL: u64 = 1 << 59;
const IOTLB_IVT: u64 = 1 << 63;
const IOTLB_GLOBAL: u64 = 1 << 60;
const IOTLB_ACTUAL_MASK: u64 = 3 << 57;
const IOTLB_ACTUAL_GLOBAL: u64 = 1 << 57;
const CAP_SAGAW_39: u64 = 1 << 9;
const FSTS_PPF: u32 = 1 << 1;
const FRCD_FAULT: u64 = 1 << 63;
const FRCD_TYPE: u64 = 1 << 62;
const PHYSICAL_LIMIT: u64 = 1 << 39;
const MAPPED_REGISTER_BYTES: u64 = 4096;

pub trait VtdRegisterIo {
    fn read_u32(&mut self, offset: u16) -> u32;
    fn write_u32(&mut self, offset: u16, value: u32);
    fn read_u64(&mut self, offset: u16) -> u64;
    fn write_u64(&mut self, offset: u16, value: u64);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VtdOperation {
    RootPointer,
    ContextInvalidation,
    IotlbInvalidation,
    TranslationEnable,
    TranslationDisable,
    FaultClear,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VtdControllerError {
    InvalidPollBudget,
    UnsupportedVersion(u32),
    Missing39BitPageTables,
    InsufficientGuestAddressWidth(u8),
    UnsupportedFaultRecording { offset: u64, records: u16 },
    InvalidIotlbOffset(u64),
    InvalidRootAddress(u64),
    TranslationAlreadyEnabled,
    TranslationAlreadyDisabled,
    Timeout(VtdOperation),
    UnexpectedInvalidationGranularity(VtdOperation),
    InconsistentFaultState,
    FaultNotPresent,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VtdFaultRecord {
    source_id: u16,
    reason: u8,
    address: u64,
    write: bool,
}

impl VtdFaultRecord {
    pub const fn source_id(self) -> u16 {
        self.source_id
    }

    pub const fn reason(self) -> u8 {
        self.reason
    }

    pub const fn address(self) -> u64 {
        self.address
    }

    pub const fn write(self) -> bool {
        self.write
    }
}

pub struct IntelVtd<I> {
    io: I,
    poll_budget: u32,
    iotlb_register: u16,
    translation_enabled: bool,
}

impl<I: VtdRegisterIo> IntelVtd<I> {
    pub fn bind(mut io: I, poll_budget: u32) -> Result<Self, VtdControllerError> {
        if poll_budget == 0 {
            return Err(VtdControllerError::InvalidPollBudget);
        }
        let version = io.read_u32(DMAR_VER_REG);
        if (version >> 4) & 0x0f == 0 {
            return Err(VtdControllerError::UnsupportedVersion(version));
        }
        let cap = io.read_u64(DMAR_CAP_REG);
        if cap & CAP_SAGAW_39 == 0 {
            return Err(VtdControllerError::Missing39BitPageTables);
        }
        let guest_address_width = (((cap >> 16) & 0x3f) as u8) + 1;
        if guest_address_width < 39 {
            return Err(VtdControllerError::InsufficientGuestAddressWidth(
                guest_address_width,
            ));
        }
        let fault_offset = ((cap >> 24) & 0x03ff) * 16;
        let fault_records = (((cap >> 40) & 0xff) as u16) + 1;
        if fault_offset != u64::from(DMAR_FRCD_LOW_REG) || fault_records != 1 {
            return Err(VtdControllerError::UnsupportedFaultRecording {
                offset: fault_offset,
                records: fault_records,
            });
        }
        let ecap = io.read_u64(DMAR_ECAP_REG);
        let iotlb_base = (ecap >> 8) & 0x03ff;
        let iotlb_offset = iotlb_base * 16 + 8;
        if iotlb_base == 0 || iotlb_offset > MAPPED_REGISTER_BYTES - 8 {
            return Err(VtdControllerError::InvalidIotlbOffset(iotlb_offset));
        }
        let iotlb_register = u16::try_from(iotlb_offset)
            .map_err(|_| VtdControllerError::InvalidIotlbOffset(iotlb_offset))?;
        let translation_enabled = io.read_u32(DMAR_GSTS_REG) & GSTS_TES != 0;
        Ok(Self {
            io,
            poll_budget,
            iotlb_register,
            translation_enabled,
        })
    }

    pub fn activate(&mut self, root_address: u64) -> Result<(), VtdControllerError> {
        if self.translation_enabled {
            return Err(VtdControllerError::TranslationAlreadyEnabled);
        }
        if root_address & 0xfff != 0 || root_address >= PHYSICAL_LIMIT {
            return Err(VtdControllerError::InvalidRootAddress(root_address));
        }
        self.io.write_u64(DMAR_RTADDR_REG, root_address);
        self.io.write_u32(DMAR_GCMD_REG, GCMD_SRTP);
        self.poll_status(GSTS_RTPS, true, VtdOperation::RootPointer)?;
        self.invalidate_context()?;
        self.invalidate_iotlb()?;
        self.io.write_u32(DMAR_GCMD_REG, GCMD_TE);
        self.poll_status(GSTS_TES, true, VtdOperation::TranslationEnable)?;
        self.translation_enabled = true;
        Ok(())
    }

    pub fn invalidate(&mut self) -> Result<(), VtdControllerError> {
        self.invalidate_context()?;
        self.invalidate_iotlb()
    }

    pub fn disable(&mut self) -> Result<(), VtdControllerError> {
        if !self.translation_enabled {
            return Err(VtdControllerError::TranslationAlreadyDisabled);
        }
        self.io.write_u32(DMAR_GCMD_REG, 0);
        self.poll_status(GSTS_TES, false, VtdOperation::TranslationDisable)?;
        self.translation_enabled = false;
        Ok(())
    }

    pub fn fault_record(&mut self) -> Result<Option<VtdFaultRecord>, VtdControllerError> {
        let pending = self.io.read_u32(DMAR_FSTS_REG) & FSTS_PPF != 0;
        let high = self.io.read_u64(DMAR_FRCD_HIGH_REG);
        let valid = high & FRCD_FAULT != 0;
        if pending != valid {
            return Err(VtdControllerError::InconsistentFaultState);
        }
        if !valid {
            return Ok(None);
        }
        let low = self.io.read_u64(DMAR_FRCD_LOW_REG);
        Ok(Some(VtdFaultRecord {
            source_id: high as u16,
            reason: ((high >> 32) & 0xff) as u8,
            address: low & !0xfff,
            write: high & FRCD_TYPE == 0,
        }))
    }

    pub fn clear_fault(&mut self) -> Result<(), VtdControllerError> {
        if self.fault_record()?.is_none() {
            return Err(VtdControllerError::FaultNotPresent);
        }
        self.io.write_u64(DMAR_FRCD_HIGH_REG, FRCD_FAULT);
        for _ in 0..self.poll_budget {
            if self.io.read_u32(DMAR_FSTS_REG) & FSTS_PPF == 0 {
                return Ok(());
            }
        }
        Err(VtdControllerError::Timeout(VtdOperation::FaultClear))
    }

    pub fn into_io(self) -> I {
        self.io
    }

    fn invalidate_context(&mut self) -> Result<(), VtdControllerError> {
        self.io.write_u64(DMAR_CCMD_REG, CCMD_ICC | CCMD_GLOBAL);
        for _ in 0..self.poll_budget {
            let command = self.io.read_u64(DMAR_CCMD_REG);
            if command & CCMD_ICC == 0 {
                if command & CCMD_ACTUAL_MASK != CCMD_ACTUAL_GLOBAL {
                    return Err(VtdControllerError::UnexpectedInvalidationGranularity(
                        VtdOperation::ContextInvalidation,
                    ));
                }
                return Ok(());
            }
        }
        Err(VtdControllerError::Timeout(
            VtdOperation::ContextInvalidation,
        ))
    }

    fn invalidate_iotlb(&mut self) -> Result<(), VtdControllerError> {
        self.io
            .write_u64(self.iotlb_register, IOTLB_IVT | IOTLB_GLOBAL);
        for _ in 0..self.poll_budget {
            let command = self.io.read_u64(self.iotlb_register);
            if command & IOTLB_IVT == 0 {
                if command & IOTLB_ACTUAL_MASK != IOTLB_ACTUAL_GLOBAL {
                    return Err(VtdControllerError::UnexpectedInvalidationGranularity(
                        VtdOperation::IotlbInvalidation,
                    ));
                }
                return Ok(());
            }
        }
        Err(VtdControllerError::Timeout(VtdOperation::IotlbInvalidation))
    }

    fn poll_status(
        &mut self,
        mask: u32,
        set: bool,
        operation: VtdOperation,
    ) -> Result<(), VtdControllerError> {
        for _ in 0..self.poll_budget {
            if (self.io.read_u32(DMAR_GSTS_REG) & mask != 0) == set {
                return Ok(());
            }
        }
        Err(VtdControllerError::Timeout(operation))
    }
}

pub struct VolatileVtdMmio {
    base: *mut u8,
}

impl VolatileVtdMmio {
    /// Bind one mapped VT-d register page.
    ///
    /// # Safety
    ///
    /// `base` must remain a writable, uncached mapping of the first 4 KiB VT-d
    /// register page for the lifetime of this value.
    pub unsafe fn new(base: *mut u8) -> Option<Self> {
        if base.is_null() || !(base as usize).is_multiple_of(4096) {
            None
        } else {
            Some(Self { base })
        }
    }
}

impl VtdRegisterIo for VolatileVtdMmio {
    fn read_u32(&mut self, offset: u16) -> u32 {
        // SAFETY: construction validates the mapped base; register constants
        // are naturally aligned and remain inside the VT-d page.
        unsafe { ptr::read_volatile(self.base.add(usize::from(offset)).cast::<u32>()) }
    }

    fn write_u32(&mut self, offset: u16, value: u32) {
        // SAFETY: same mapped-register contract as `read_u32`.
        unsafe {
            ptr::write_volatile(self.base.add(usize::from(offset)).cast::<u32>(), value);
        }
    }

    fn read_u64(&mut self, offset: u16) -> u64 {
        // SAFETY: same mapped-register contract as `read_u32`.
        unsafe { ptr::read_volatile(self.base.add(usize::from(offset)).cast::<u64>()) }
    }

    fn write_u64(&mut self, offset: u16, value: u64) {
        // SAFETY: same mapped-register contract as `read_u32`.
        unsafe {
            ptr::write_volatile(self.base.add(usize::from(offset)).cast::<u64>(), value);
        }
    }
}
