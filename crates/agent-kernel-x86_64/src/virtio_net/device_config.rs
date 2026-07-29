//! Volatile Virtio network Device Configuration adapter.
//!
//! This x86 MMIO view reads only the six-byte MAC field from a validated
//! Device Configuration BAR region.

use core::ptr;

use crate::pci::VirtioPciBarRegion;

pub trait VirtioNetDeviceConfigIo {
    fn read_mac(&mut self) -> [u8; 6];
}

pub struct VolatileVirtioNetDeviceConfig {
    base: *mut u8,
}

impl VolatileVirtioNetDeviceConfig {
    /// Binds one Device Configuration capability region.
    ///
    /// # Safety
    ///
    /// `mapped_bar` must remain a readable device mapping for `mapped_bytes`
    /// while this adapter exists.
    pub unsafe fn bind(
        mapped_bar: *mut u8,
        mapped_bytes: usize,
        region: VirtioPciBarRegion,
    ) -> Result<Self, VirtioNetDeviceConfigError> {
        if mapped_bar.is_null() {
            return Err(VirtioNetDeviceConfigError::NullMappedBar);
        }
        if region.length() < 6 {
            return Err(VirtioNetDeviceConfigError::RegionTooSmall {
                required: 6,
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
            .ok_or(VirtioNetDeviceConfigError::MappedAddressOverflow)?;
        if required > mapped_bytes {
            return Err(VirtioNetDeviceConfigError::MappedBarTooSmall {
                required,
                actual: mapped_bytes,
            });
        }
        let address = (mapped_bar as usize)
            .checked_add(region.offset() as usize)
            .ok_or(VirtioNetDeviceConfigError::MappedAddressOverflow)?;
        Ok(Self {
            base: address as *mut u8,
        })
    }
}

impl VirtioNetDeviceConfigIo for VolatileVirtioNetDeviceConfig {
    fn read_mac(&mut self) -> [u8; 6] {
        let mut mac = [0; 6];
        for (index, byte) in mac.iter_mut().enumerate() {
            *byte = unsafe { ptr::read_volatile(self.base.add(index).cast::<u8>()) };
        }
        mac
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioNetDeviceConfigError {
    NullMappedBar,
    MappedAddressOverflow,
    MappedBarTooSmall { required: usize, actual: usize },
    RegionTooSmall { required: u32, actual: u32 },
}
