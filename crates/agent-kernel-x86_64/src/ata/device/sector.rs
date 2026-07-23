//! Single-sector LBA48 read, write, and cache-flush commands.

use super::{
    AtaPioDevice, ERROR_FEATURES, LBA_HIGH, LBA_LOW, LBA_MID, SECTOR_COUNT, STATUS_COMMAND,
};
use crate::ata::{
    AtaPioError, AtaRegisterIo, ATA_COMMAND_FLUSH_EXT, ATA_COMMAND_READ_EXT, ATA_COMMAND_WRITE_EXT,
    ATA_SECTOR_BYTES,
};

impl<I: AtaRegisterIo> AtaPioDevice<I> {
    pub fn read_sector(
        &mut self,
        lba: u64,
        output: &mut [u8; ATA_SECTOR_BYTES],
    ) -> Result<(), AtaPioError> {
        self.validate_lba(lba)?;
        self.prepare_lba48(lba)?;
        self.write_register(STATUS_COMMAND, ATA_COMMAND_READ_EXT);
        self.wait_data_request()?;
        let (words, remainder) = output.as_chunks_mut::<2>();
        debug_assert!(remainder.is_empty());
        for chunk in words {
            chunk.copy_from_slice(&self.read_data().to_le_bytes());
        }
        self.wait_command_complete()?;
        Ok(())
    }

    pub fn write_sector(
        &mut self,
        lba: u64,
        input: &[u8; ATA_SECTOR_BYTES],
    ) -> Result<(), AtaPioError> {
        self.validate_lba(lba)?;
        self.prepare_lba48(lba)?;
        self.write_register(STATUS_COMMAND, ATA_COMMAND_WRITE_EXT);
        self.wait_data_request()?;
        let (words, remainder) = input.as_chunks::<2>();
        debug_assert!(remainder.is_empty());
        for word in words {
            self.write_data(u16::from_le_bytes(*word));
        }
        self.wait_command_complete()?;
        Ok(())
    }

    pub fn flush_cache(&mut self) -> Result<(), AtaPioError> {
        if self.identity.is_none() {
            return Err(AtaPioError::DeviceNotIdentified);
        }
        self.select_drive();
        self.wait_not_busy()?;
        self.write_register(STATUS_COMMAND, ATA_COMMAND_FLUSH_EXT);
        self.wait_not_busy()?;
        Ok(())
    }

    fn validate_lba(&self, lba: u64) -> Result<(), AtaPioError> {
        let identity = self.identity.ok_or(AtaPioError::DeviceNotIdentified)?;
        if lba >= identity.sector_count() {
            return Err(AtaPioError::LbaOutOfRange {
                lba,
                sector_count: identity.sector_count(),
            });
        }
        Ok(())
    }

    fn prepare_lba48(&mut self, lba: u64) -> Result<(), AtaPioError> {
        self.select_drive();
        self.wait_not_busy()?;
        let bytes = lba.to_le_bytes();
        self.write_register(ERROR_FEATURES, 0);
        self.write_register(SECTOR_COUNT, 0);
        self.write_register(LBA_LOW, bytes[3]);
        self.write_register(LBA_MID, bytes[4]);
        self.write_register(LBA_HIGH, bytes[5]);
        self.write_register(ERROR_FEATURES, 0);
        self.write_register(SECTOR_COUNT, 1);
        self.write_register(LBA_LOW, bytes[0]);
        self.write_register(LBA_MID, bytes[1]);
        self.write_register(LBA_HIGH, bytes[2]);
        Ok(())
    }
}
