//! Native ATA binding for the V13 semantic durable-state HAL.
//!
//! This adapter owns A/B sector mapping, a caller-provided staging buffer,
//! ordered region writes, device flush epochs, and recovered-head progression.

mod backend;
mod binding;
mod head;

pub use backend::{AtaDurableBackendInitError, AtaDurableStateBackend};
pub use binding::{AtaDurableBinding, AtaDurableBindingError};
pub use head::{AtaDurableHead, AtaDurableHeadBindError};

use agent_kernel_hal::DURABLE_SLOT_BYTES;

use super::ATA_SECTOR_BYTES;

pub const ATA_DURABLE_SLOT_SECTORS: u64 = (DURABLE_SLOT_BYTES / ATA_SECTOR_BYTES) as u64;
pub const ATA_DURABLE_RANGE_SECTORS: u64 = ATA_DURABLE_SLOT_SECTORS * 2;
