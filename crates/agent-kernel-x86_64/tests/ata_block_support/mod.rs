use std::collections::BTreeMap;

use agent_kernel_x86_64::ata::{AtaBlockDevice, AtaDeviceIdentity, AtaPioError, ATA_SECTOR_BYTES};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SectorOperation {
    Read(u64),
    Write(u64),
    Flush,
}

pub struct SectorDevice {
    identity: AtaDeviceIdentity,
    volatile_sectors: BTreeMap<u64, [u8; ATA_SECTOR_BYTES]>,
    durable_sectors: BTreeMap<u64, [u8; ATA_SECTOR_BYTES]>,
    operations: Vec<SectorOperation>,
    fail_at: Option<usize>,
    failure: AtaPioError,
}

impl SectorDevice {
    pub fn new(identity: AtaDeviceIdentity) -> Self {
        Self {
            identity,
            volatile_sectors: BTreeMap::new(),
            durable_sectors: BTreeMap::new(),
            operations: Vec::new(),
            fail_at: None,
            failure: AtaPioError::BusyTimeout,
        }
    }

    pub fn failing_at(identity: AtaDeviceIdentity, operation: usize, failure: AtaPioError) -> Self {
        let mut device = Self::new(identity);
        device.fail_at = Some(operation);
        device.failure = failure;
        device
    }

    pub fn operations(&self) -> &[SectorOperation] {
        &self.operations
    }

    pub fn sector(&self, lba: u64) -> [u8; ATA_SECTOR_BYTES] {
        self.volatile_sectors
            .get(&lba)
            .copied()
            .unwrap_or([0; ATA_SECTOR_BYTES])
    }

    #[allow(dead_code)]
    pub fn simulate_power_loss(&mut self) {
        self.volatile_sectors = self.durable_sectors.clone();
    }

    fn begin(&mut self, operation: SectorOperation) -> Result<(), AtaPioError> {
        self.operations.push(operation);
        if self.fail_at == Some(self.operations.len()) {
            self.fail_at = None;
            Err(self.failure)
        } else {
            Ok(())
        }
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
        self.begin(SectorOperation::Read(lba))?;
        *output = self.sector(lba);
        Ok(())
    }

    fn write_sector(
        &mut self,
        lba: u64,
        input: &[u8; ATA_SECTOR_BYTES],
    ) -> Result<(), AtaPioError> {
        self.begin(SectorOperation::Write(lba))?;
        self.volatile_sectors.insert(lba, *input);
        Ok(())
    }

    fn flush_cache(&mut self) -> Result<(), AtaPioError> {
        self.begin(SectorOperation::Flush)?;
        self.durable_sectors = self.volatile_sectors.clone();
        Ok(())
    }
}
