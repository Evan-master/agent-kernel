//! Sector-level boundary shared by the native transport and durable adapter.

use super::{AtaDeviceIdentity, AtaPioDevice, AtaPioError, AtaRegisterIo, ATA_SECTOR_BYTES};

pub trait AtaBlockDevice {
    fn identity(&self) -> Option<AtaDeviceIdentity>;

    fn read_sector(
        &mut self,
        lba: u64,
        output: &mut [u8; ATA_SECTOR_BYTES],
    ) -> Result<(), AtaPioError>;

    fn write_sector(&mut self, lba: u64, input: &[u8; ATA_SECTOR_BYTES])
        -> Result<(), AtaPioError>;

    fn flush_cache(&mut self) -> Result<(), AtaPioError>;
}

impl<I: AtaRegisterIo> AtaBlockDevice for AtaPioDevice<I> {
    fn identity(&self) -> Option<AtaDeviceIdentity> {
        AtaPioDevice::identity(self)
    }

    fn read_sector(
        &mut self,
        lba: u64,
        output: &mut [u8; ATA_SECTOR_BYTES],
    ) -> Result<(), AtaPioError> {
        AtaPioDevice::read_sector(self, lba, output)
    }

    fn write_sector(
        &mut self,
        lba: u64,
        input: &[u8; ATA_SECTOR_BYTES],
    ) -> Result<(), AtaPioError> {
        AtaPioDevice::write_sector(self, lba, input)
    }

    fn flush_cache(&mut self) -> Result<(), AtaPioError> {
        AtaPioDevice::flush_cache(self)
    }
}
