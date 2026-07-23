//! Bounded ATA PIO storage transport for the native durable-state profile.
//!
//! The x86 architecture layer owns task-file programming, identity validation,
//! finite status polling, and 512-byte sector transfer. Kernel authority and
//! durable capsule semantics remain in Core and HAL.

mod block;
mod config;
mod device;
mod durable;
mod error;
mod io;

pub use block::AtaBlockDevice;
pub use config::{AtaDrive, AtaPioConfig, AtaPioConfigError};
pub use device::{AtaDeviceIdentity, AtaPioDevice};
pub use durable::{
    AtaDurableBackendInitError, AtaDurableBinding, AtaDurableBindingError, AtaDurableHead,
    AtaDurableHeadBindError, AtaDurableStateBackend, NativeAtaDurableBootState,
    NativeAtaDurableCommitError, NativeAtaDurableConfig, NativeAtaDurableConfigError,
    NativeAtaDurableInitError, NativeAtaDurableSession, ATA_DURABLE_RANGE_SECTORS,
    ATA_DURABLE_SLOT_SECTORS,
};
pub use error::AtaPioError;
pub use io::AtaRegisterIo;

pub const ATA_SECTOR_BYTES: usize = 512;
pub const ATA_LBA48_SECTOR_LIMIT: u64 = 1_u64 << 48;
pub const ATA_COMMAND_READ_EXT: u8 = 0x24;
pub const ATA_COMMAND_WRITE_EXT: u8 = 0x34;
pub const ATA_COMMAND_FLUSH_EXT: u8 = 0xea;
pub const ATA_COMMAND_IDENTIFY: u8 = 0xec;

const ATA_STATUS_ERROR: u8 = 1 << 0;
const ATA_STATUS_DATA_REQUEST: u8 = 1 << 3;
const ATA_STATUS_DEVICE_FAULT: u8 = 1 << 5;
const ATA_STATUS_BUSY: u8 = 1 << 7;
