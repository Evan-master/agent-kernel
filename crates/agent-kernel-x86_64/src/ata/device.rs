//! Identified ATA PIO device and bounded task-file operations.

mod identify;
mod sector;

use super::{
    AtaPioConfig, AtaPioError, AtaRegisterIo, ATA_STATUS_BUSY, ATA_STATUS_DATA_REQUEST,
    ATA_STATUS_DEVICE_FAULT, ATA_STATUS_ERROR,
};

const DATA: u16 = 0;
const ERROR_FEATURES: u16 = 1;
const SECTOR_COUNT: u16 = 2;
const LBA_LOW: u16 = 3;
const LBA_MID: u16 = 4;
const LBA_HIGH: u16 = 5;
const DEVICE: u16 = 6;
const STATUS_COMMAND: u16 = 7;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AtaDeviceIdentity {
    sector_count: u64,
}

impl AtaDeviceIdentity {
    pub const fn new(sector_count: u64) -> Option<Self> {
        if sector_count == 0 || sector_count > super::ATA_LBA48_SECTOR_LIMIT {
            None
        } else {
            Some(Self { sector_count })
        }
    }

    pub const fn sector_count(self) -> u64 {
        self.sector_count
    }

    pub const fn logical_sector_bytes(self) -> usize {
        super::ATA_SECTOR_BYTES
    }
}

pub struct AtaPioDevice<I> {
    config: AtaPioConfig,
    io: I,
    identity: Option<AtaDeviceIdentity>,
}

impl<I> AtaPioDevice<I> {
    pub const fn new(config: AtaPioConfig, io: I) -> Self {
        Self {
            config,
            io,
            identity: None,
        }
    }

    pub const fn config(&self) -> AtaPioConfig {
        self.config
    }

    pub const fn identity(&self) -> Option<AtaDeviceIdentity> {
        self.identity
    }

    pub const fn io(&self) -> &I {
        &self.io
    }

    pub fn io_mut(&mut self) -> &mut I {
        &mut self.io
    }

    pub fn into_io(self) -> I {
        self.io
    }
}

impl<I: AtaRegisterIo> AtaPioDevice<I> {
    fn select_drive(&mut self) {
        self.write_register(DEVICE, self.config.drive().device_select());
        for _ in 0..4 {
            self.io.read_u8(self.config.control_base());
        }
    }

    fn wait_not_busy(&mut self) -> Result<u8, AtaPioError> {
        for _ in 0..self.config.poll_budget() {
            let status = self.read_status();
            Self::validate_presence(status)?;
            if status & ATA_STATUS_BUSY == 0 {
                self.validate_fault(status)?;
                return Ok(status);
            }
        }
        Err(AtaPioError::BusyTimeout)
    }

    fn wait_data_request(&mut self) -> Result<u8, AtaPioError> {
        for _ in 0..self.config.poll_budget() {
            let status = self.read_status();
            Self::validate_presence(status)?;
            if status & ATA_STATUS_BUSY == 0 {
                self.validate_fault(status)?;
                if status & ATA_STATUS_DATA_REQUEST != 0 {
                    return Ok(status);
                }
            }
        }
        Err(AtaPioError::DataRequestTimeout)
    }

    fn wait_command_complete(&mut self) -> Result<u8, AtaPioError> {
        for _ in 0..self.config.poll_budget() {
            let status = self.read_status();
            Self::validate_presence(status)?;
            if status & ATA_STATUS_BUSY == 0 {
                self.validate_fault(status)?;
                if status & ATA_STATUS_DATA_REQUEST == 0 {
                    return Ok(status);
                }
            }
        }
        Err(AtaPioError::CommandCompletionTimeout)
    }

    fn validate_fault(&mut self, status: u8) -> Result<(), AtaPioError> {
        if status & (ATA_STATUS_ERROR | ATA_STATUS_DEVICE_FAULT) == 0 {
            return Ok(());
        }
        Err(AtaPioError::DeviceFault {
            status,
            error: self.read_register(ERROR_FEATURES),
        })
    }

    const fn validate_presence(status: u8) -> Result<(), AtaPioError> {
        if status == 0 || status == u8::MAX {
            Err(AtaPioError::NoDevice)
        } else {
            Ok(())
        }
    }

    fn read_status(&mut self) -> u8 {
        self.read_register(STATUS_COMMAND)
    }

    fn read_register(&mut self, offset: u16) -> u8 {
        self.io.read_u8(self.config.port(offset))
    }

    fn write_register(&mut self, offset: u16, value: u8) {
        self.io.write_u8(self.config.port(offset), value);
    }

    fn read_data(&mut self) -> u16 {
        self.io.read_u16(self.config.port(DATA))
    }

    fn write_data(&mut self, value: u16) {
        self.io.write_u16(self.config.port(DATA), value);
    }
}
