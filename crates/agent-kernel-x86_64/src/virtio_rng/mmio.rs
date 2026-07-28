//! Volatile modern Virtio PCI capability-region adapters.
//!
//! This x86 architecture module translates validated BAR-relative Common,
//! Notify, and ISR regions into width-aware volatile access. Construction
//! rejects short mappings and invalid natural alignment before any MMIO.

use core::ptr;

use crate::pci::VirtioPciBarRegion;

use super::{VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo};

const COMMON_CONFIG_BYTES: u32 = 0x38;
const NOTIFY_CONFIG_BYTES: u32 = 2;
const ISR_CONFIG_BYTES: u32 = 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioMmioRegionKind {
    Common,
    Notify,
    Isr,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioMmioError {
    NullMappedBar,
    MappedAddressOverflow,
    MappedBarTooSmall {
        required: usize,
        actual: usize,
    },
    RegionTooSmall {
        region: VirtioMmioRegionKind,
        required: u32,
        actual: u32,
    },
    RegionUnaligned {
        region: VirtioMmioRegionKind,
        address: usize,
        required: usize,
    },
}

pub struct VolatileVirtioCommonConfig {
    base: *mut u8,
    length: u32,
}

impl VolatileVirtioCommonConfig {
    /// Binds one Common Configuration capability region.
    ///
    /// # Safety
    ///
    /// `mapped_bar` must remain a writable device mapping for `mapped_bytes`
    /// while this adapter exists.
    pub unsafe fn bind(
        mapped_bar: *mut u8,
        mapped_bytes: usize,
        region: VirtioPciBarRegion,
    ) -> Result<Self, VirtioMmioError> {
        let (base, length) = bind_region(
            mapped_bar,
            mapped_bytes,
            region,
            COMMON_CONFIG_BYTES,
            8,
            VirtioMmioRegionKind::Common,
        )?;
        Ok(Self { base, length })
    }

    fn contains(&self, offset: u16, width: u32) -> bool {
        u32::from(offset)
            .checked_add(width)
            .is_some_and(|end| end <= self.length)
    }
}

impl VirtioCommonConfigIo for VolatileVirtioCommonConfig {
    fn read_u8(&mut self, offset: u16) -> u8 {
        assert!(self.contains(offset, 1));
        unsafe { ptr::read_volatile(self.base.add(usize::from(offset)).cast::<u8>()) }
    }

    fn read_u16(&mut self, offset: u16) -> u16 {
        assert!(self.contains(offset, 2) && offset.is_multiple_of(2));
        unsafe { ptr::read_volatile(self.base.add(usize::from(offset)).cast::<u16>()) }
    }

    fn read_u32(&mut self, offset: u16) -> u32 {
        assert!(self.contains(offset, 4) && offset.is_multiple_of(4));
        unsafe { ptr::read_volatile(self.base.add(usize::from(offset)).cast::<u32>()) }
    }

    fn write_u8(&mut self, offset: u16, value: u8) {
        assert!(self.contains(offset, 1));
        unsafe {
            ptr::write_volatile(self.base.add(usize::from(offset)).cast::<u8>(), value);
        }
    }

    fn write_u16(&mut self, offset: u16, value: u16) {
        assert!(self.contains(offset, 2) && offset.is_multiple_of(2));
        unsafe {
            ptr::write_volatile(self.base.add(usize::from(offset)).cast::<u16>(), value);
        }
    }

    fn write_u32(&mut self, offset: u16, value: u32) {
        assert!(self.contains(offset, 4) && offset.is_multiple_of(4));
        unsafe {
            ptr::write_volatile(self.base.add(usize::from(offset)).cast::<u32>(), value);
        }
    }

    fn write_u64(&mut self, offset: u16, value: u64) {
        assert!(self.contains(offset, 8) && offset.is_multiple_of(8));
        unsafe {
            ptr::write_volatile(self.base.add(usize::from(offset)).cast::<u64>(), value);
        }
    }
}

pub struct VolatileVirtioNotify {
    base: *mut u8,
    length: u32,
    offset_multiplier: u32,
}

impl VolatileVirtioNotify {
    /// Binds one Notify capability region.
    ///
    /// # Safety
    ///
    /// The BAR mapping contract matches `VolatileVirtioCommonConfig::bind`.
    pub unsafe fn bind(
        mapped_bar: *mut u8,
        mapped_bytes: usize,
        region: VirtioPciBarRegion,
        offset_multiplier: u32,
    ) -> Result<Self, VirtioMmioError> {
        let (base, length) = bind_region(
            mapped_bar,
            mapped_bytes,
            region,
            NOTIFY_CONFIG_BYTES,
            2,
            VirtioMmioRegionKind::Notify,
        )?;
        Ok(Self {
            base,
            length,
            offset_multiplier,
        })
    }
}

impl VirtioNotifyIo for VolatileVirtioNotify {
    fn region_bytes(&self) -> u32 {
        self.length
    }

    fn offset_multiplier(&self) -> u32 {
        self.offset_multiplier
    }

    fn write_u16(&mut self, byte_offset: u32, value: u16) {
        assert!(
            byte_offset.is_multiple_of(2)
                && byte_offset
                    .checked_add(2)
                    .is_some_and(|end| end <= self.length)
        );
        unsafe {
            ptr::write_volatile(self.base.add(byte_offset as usize).cast::<u16>(), value);
        }
    }
}

pub struct VolatileVirtioIsr {
    base: *mut u8,
}

impl VolatileVirtioIsr {
    /// Binds one ISR Status capability region.
    ///
    /// # Safety
    ///
    /// The BAR mapping contract matches `VolatileVirtioCommonConfig::bind`.
    pub unsafe fn bind(
        mapped_bar: *mut u8,
        mapped_bytes: usize,
        region: VirtioPciBarRegion,
    ) -> Result<Self, VirtioMmioError> {
        let (base, _) = bind_region(
            mapped_bar,
            mapped_bytes,
            region,
            ISR_CONFIG_BYTES,
            1,
            VirtioMmioRegionKind::Isr,
        )?;
        Ok(Self { base })
    }
}

impl VirtioIsrIo for VolatileVirtioIsr {
    fn read_and_acknowledge(&mut self) -> u8 {
        unsafe { ptr::read_volatile(self.base.cast::<u8>()) }
    }
}

fn bind_region(
    mapped_bar: *mut u8,
    mapped_bytes: usize,
    region: VirtioPciBarRegion,
    minimum_length: u32,
    alignment: usize,
    kind: VirtioMmioRegionKind,
) -> Result<(*mut u8, u32), VirtioMmioError> {
    if mapped_bar.is_null() {
        return Err(VirtioMmioError::NullMappedBar);
    }
    if region.length() < minimum_length {
        return Err(VirtioMmioError::RegionTooSmall {
            region: kind,
            required: minimum_length,
            actual: region.length(),
        });
    }
    let required = usize::try_from(region.offset())
        .ok()
        .and_then(|offset| {
            usize::try_from(region.length())
                .ok()
                .and_then(|length| offset.checked_add(length))
        })
        .ok_or(VirtioMmioError::MappedAddressOverflow)?;
    if required > mapped_bytes {
        return Err(VirtioMmioError::MappedBarTooSmall {
            required,
            actual: mapped_bytes,
        });
    }
    let address = (mapped_bar as usize)
        .checked_add(region.offset() as usize)
        .ok_or(VirtioMmioError::MappedAddressOverflow)?;
    if !address.is_multiple_of(alignment) {
        return Err(VirtioMmioError::RegionUnaligned {
            region: kind,
            address,
            required: alignment,
        });
    }
    Ok((address as *mut u8, region.length()))
}
