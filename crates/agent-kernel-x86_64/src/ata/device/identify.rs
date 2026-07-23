//! ATA identity command and fixed logical-sector profile validation.

use super::{AtaDeviceIdentity, AtaPioDevice, LBA_HIGH, LBA_LOW, LBA_MID, SECTOR_COUNT};
use crate::ata::{
    AtaPioError, AtaRegisterIo, ATA_COMMAND_IDENTIFY, ATA_LBA48_SECTOR_LIMIT, ATA_SECTOR_BYTES,
    ATA_STATUS_DATA_REQUEST,
};

const IDENTIFY_WORDS: usize = 256;
const LBA48_SUPPORT_WORD: usize = 83;
const LBA48_SUPPORT_BIT: u16 = 1 << 10;
const SECTOR_SIZE_INFORMATION_WORD: usize = 106;
const LOGICAL_SECTOR_LONGER_BIT: u16 = 1 << 12;
const SECTOR_SIZE_VALID_MASK: u16 = 0xc000;
const SECTOR_SIZE_VALID_VALUE: u16 = 0x4000;

impl<I: AtaRegisterIo> AtaPioDevice<I> {
    pub fn identify(&mut self) -> Result<AtaDeviceIdentity, AtaPioError> {
        self.select_drive();
        self.wait_not_busy()?;
        self.write_register(SECTOR_COUNT, 0);
        self.write_register(LBA_LOW, 0);
        self.write_register(LBA_MID, 0);
        self.write_register(LBA_HIGH, 0);
        self.write_register(super::STATUS_COMMAND, ATA_COMMAND_IDENTIFY);

        let status = self.read_status();
        Self::validate_presence(status)?;
        self.wait_not_busy()?;
        let lba_mid = self.read_register(LBA_MID);
        let lba_high = self.read_register(LBA_HIGH);
        if lba_mid != 0 || lba_high != 0 {
            return Err(AtaPioError::DeviceSignatureUnsupported { lba_mid, lba_high });
        }
        let status = self.wait_data_request()?;
        debug_assert_ne!(status & ATA_STATUS_DATA_REQUEST, 0);

        let mut words = [0_u16; IDENTIFY_WORDS];
        for word in &mut words {
            *word = self.read_data();
        }
        self.wait_command_complete()?;
        let identity = parse_identity(&words)?;
        self.identity = Some(identity);
        Ok(identity)
    }
}

fn parse_identity(words: &[u16; IDENTIFY_WORDS]) -> Result<AtaDeviceIdentity, AtaPioError> {
    if words[LBA48_SUPPORT_WORD] & LBA48_SUPPORT_BIT == 0 {
        return Err(AtaPioError::Lba48Unsupported);
    }
    let sector_bytes = logical_sector_bytes(words);
    if sector_bytes != ATA_SECTOR_BYTES as u32 {
        return Err(AtaPioError::LogicalSectorSizeUnsupported {
            bytes: sector_bytes,
        });
    }
    let sector_count = u64::from(words[100])
        | (u64::from(words[101]) << 16)
        | (u64::from(words[102]) << 32)
        | (u64::from(words[103]) << 48);
    if sector_count == 0 {
        return Err(AtaPioError::ZeroSectorCapacity);
    }
    if sector_count > ATA_LBA48_SECTOR_LIMIT {
        return Err(AtaPioError::Lba48CapacityInvalid { sector_count });
    }
    AtaDeviceIdentity::new(sector_count).ok_or(AtaPioError::ZeroSectorCapacity)
}

fn logical_sector_bytes(words: &[u16; IDENTIFY_WORDS]) -> u32 {
    let size_information = words[SECTOR_SIZE_INFORMATION_WORD];
    let is_valid = size_information & SECTOR_SIZE_VALID_MASK == SECTOR_SIZE_VALID_VALUE;
    if is_valid && size_information & LOGICAL_SECTOR_LONGER_BIT != 0 {
        let logical_words = u32::from(words[117]) | (u32::from(words[118]) << 16);
        logical_words.saturating_mul(2)
    } else {
        ATA_SECTOR_BYTES as u32
    }
}
