use std::collections::BTreeMap;

use agent_kernel_x86_64::ata::{AtaBlockDevice, AtaDeviceIdentity, AtaPioError, ATA_SECTOR_BYTES};

pub struct SectorDevice {
    identity: AtaDeviceIdentity,
    volatile: BTreeMap<u64, [u8; ATA_SECTOR_BYTES]>,
    durable: BTreeMap<u64, [u8; ATA_SECTOR_BYTES]>,
}

impl SectorDevice {
    pub fn new(identity: AtaDeviceIdentity) -> Self {
        Self {
            identity,
            volatile: BTreeMap::new(),
            durable: BTreeMap::new(),
        }
    }

    pub fn simulate_power_loss(&mut self) {
        self.volatile = self.durable.clone();
    }
}

impl AtaBlockDevice for SectorDevice {
    fn identity(&self) -> Option<AtaDeviceIdentity> {
        Some(self.identity)
    }

    fn read_sector(
        &mut self,
        lba: u64,
        output: &mut [u8; ATA_SECTOR_BYTES],
    ) -> Result<(), AtaPioError> {
        *output = self
            .volatile
            .get(&lba)
            .copied()
            .unwrap_or([0; ATA_SECTOR_BYTES]);
        Ok(())
    }

    fn write_sector(
        &mut self,
        lba: u64,
        input: &[u8; ATA_SECTOR_BYTES],
    ) -> Result<(), AtaPioError> {
        self.volatile.insert(lba, *input);
        Ok(())
    }

    fn flush_cache(&mut self) -> Result<(), AtaPioError> {
        self.durable = self.volatile.clone();
        Ok(())
    }
}
