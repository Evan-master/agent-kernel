//! Volatile access to one boot-mapped TPM CRB window.
//!
//! The x86 machine layer translates validated physical addresses into one
//! supervisor-only mapping; construction carries the mapping-lifetime proof.

use super::CrbIo;

const REGISTER_ALIGNMENT: u64 = 4;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CrbMmioError {
    InvalidWindow,
    UnalignedRegister { address: u64 },
    OutsideWindow { address: u64, length: usize },
}

/// Physical-to-virtual access for one immutable CRB mapping.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VolatileCrbIo {
    physical_base: u64,
    virtual_base: u64,
    length: usize,
}

impl VolatileCrbIo {
    /// Creates an accessor for an existing device mapping.
    ///
    /// # Safety
    ///
    /// `virtual_base..virtual_base + length` must remain mapped to
    /// `physical_base..physical_base + length` as uncached TPM device memory
    /// for the lifetime of this value. No other owner may issue unsynchronized
    /// TPM CRB transactions through the same mapping.
    pub unsafe fn new(
        physical_base: u64,
        virtual_base: u64,
        length: usize,
    ) -> Result<Self, CrbMmioError> {
        let length_u64 = u64::try_from(length).map_err(|_| CrbMmioError::InvalidWindow)?;
        if physical_base == 0
            || virtual_base == 0
            || length == 0
            || !physical_base.is_multiple_of(REGISTER_ALIGNMENT)
            || !virtual_base.is_multiple_of(REGISTER_ALIGNMENT)
            || physical_base.checked_add(length_u64).is_none()
            || virtual_base.checked_add(length_u64).is_none()
        {
            return Err(CrbMmioError::InvalidWindow);
        }
        Ok(Self {
            physical_base,
            virtual_base,
            length,
        })
    }

    pub const fn physical_base(&self) -> u64 {
        self.physical_base
    }

    pub const fn virtual_base(&self) -> u64 {
        self.virtual_base
    }

    pub const fn length(&self) -> usize {
        self.length
    }

    fn translate(&self, address: u64, length: usize) -> Result<u64, CrbMmioError> {
        let length_u64 =
            u64::try_from(length).map_err(|_| CrbMmioError::OutsideWindow { address, length })?;
        let window_length = self.length as u64;
        let window_end = self.physical_base + window_length;
        let access_end = address
            .checked_add(length_u64)
            .ok_or(CrbMmioError::OutsideWindow { address, length })?;
        if address < self.physical_base || access_end > window_end {
            return Err(CrbMmioError::OutsideWindow { address, length });
        }
        self.virtual_base
            .checked_add(address - self.physical_base)
            .ok_or(CrbMmioError::OutsideWindow { address, length })
    }
}

impl CrbIo for VolatileCrbIo {
    type Error = CrbMmioError;

    fn read_u32(&mut self, address: u64) -> Result<u32, Self::Error> {
        if !address.is_multiple_of(REGISTER_ALIGNMENT) {
            return Err(CrbMmioError::UnalignedRegister { address });
        }
        let virtual_address = self.translate(address, size_of::<u32>())?;
        // SAFETY: construction guarantees the mapping lifetime and translate
        // restricts this aligned access to the registered device window.
        Ok(unsafe { core::ptr::read_volatile(virtual_address as *const u32) })
    }

    fn write_u32(&mut self, address: u64, value: u32) -> Result<(), Self::Error> {
        if !address.is_multiple_of(REGISTER_ALIGNMENT) {
            return Err(CrbMmioError::UnalignedRegister { address });
        }
        let virtual_address = self.translate(address, size_of::<u32>())?;
        // SAFETY: construction guarantees the mapping lifetime and translate
        // restricts this aligned access to the registered device window.
        unsafe { core::ptr::write_volatile(virtual_address as *mut u32, value) };
        Ok(())
    }

    fn read_bytes(&mut self, address: u64, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let virtual_address = self.translate(address, bytes.len())?;
        for (offset, byte) in bytes.iter_mut().enumerate() {
            // SAFETY: translate validated the full range before the loop.
            *byte = unsafe { core::ptr::read_volatile((virtual_address as *const u8).add(offset)) };
        }
        Ok(())
    }

    fn write_bytes(&mut self, address: u64, bytes: &[u8]) -> Result<(), Self::Error> {
        let virtual_address = self.translate(address, bytes.len())?;
        for (offset, byte) in bytes.iter().copied().enumerate() {
            // SAFETY: translate validated the full range before the loop.
            unsafe { core::ptr::write_volatile((virtual_address as *mut u8).add(offset), byte) };
        }
        Ok(())
    }
}
