//! Validated Resource-to-device range binding for two durable slots.

use agent_kernel_core::{DurableSlot, ResourceId};

use crate::ata::{AtaDeviceIdentity, ATA_DURABLE_RANGE_SECTORS, ATA_DURABLE_SLOT_SECTORS};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AtaDurableBindingError {
    ZeroStorageResource,
    BaseLbaUnaligned {
        lba: u64,
        required_sectors: u64,
    },
    RangeOverflow,
    RangeExceedsDevice {
        required_exclusive: u64,
        sector_count: u64,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AtaDurableBinding {
    storage: ResourceId,
    base_lba: u64,
    identity: AtaDeviceIdentity,
}

impl AtaDurableBinding {
    pub const fn new(
        storage: ResourceId,
        base_lba: u64,
        identity: AtaDeviceIdentity,
    ) -> Result<Self, AtaDurableBindingError> {
        if storage.raw() == 0 {
            return Err(AtaDurableBindingError::ZeroStorageResource);
        }
        if !base_lba.is_multiple_of(ATA_DURABLE_SLOT_SECTORS) {
            return Err(AtaDurableBindingError::BaseLbaUnaligned {
                lba: base_lba,
                required_sectors: ATA_DURABLE_SLOT_SECTORS,
            });
        }
        let Some(required_exclusive) = base_lba.checked_add(ATA_DURABLE_RANGE_SECTORS) else {
            return Err(AtaDurableBindingError::RangeOverflow);
        };
        if required_exclusive > identity.sector_count() {
            return Err(AtaDurableBindingError::RangeExceedsDevice {
                required_exclusive,
                sector_count: identity.sector_count(),
            });
        }
        Ok(Self {
            storage,
            base_lba,
            identity,
        })
    }

    pub const fn storage(self) -> ResourceId {
        self.storage
    }

    pub const fn base_lba(self) -> u64 {
        self.base_lba
    }

    pub const fn identity(self) -> AtaDeviceIdentity {
        self.identity
    }

    pub const fn range_sectors(self) -> u64 {
        ATA_DURABLE_RANGE_SECTORS
    }

    pub const fn slot_lba(self, slot: DurableSlot) -> u64 {
        match slot {
            DurableSlot::A => self.base_lba,
            DurableSlot::B => self.base_lba + ATA_DURABLE_SLOT_SECTORS,
        }
    }
}
