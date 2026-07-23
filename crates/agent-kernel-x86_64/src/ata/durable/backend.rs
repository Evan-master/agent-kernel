//! State and device helpers for the ATA durable-state backend.

mod hal;

use agent_kernel_core::{DurableSlot, ResourceId};
use agent_kernel_hal::{DurableSlotTarget, DurableStateBackendError, DURABLE_SLOT_BYTES};

use super::{AtaDurableBinding, AtaDurableHead, AtaDurableHeadBindError, ATA_DURABLE_SLOT_SECTORS};
use crate::ata::{AtaBlockDevice, AtaDeviceIdentity, AtaPioError, ATA_SECTOR_BYTES};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AtaDurableBackendInitError {
    DeviceNotIdentified,
    DeviceIdentityMismatch {
        binding: AtaDeviceIdentity,
        device: AtaDeviceIdentity,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BackendPhase {
    Idle,
    HeaderDirty(DurableSlotTarget),
    HeaderFlushed(DurableSlotTarget),
    BodyDirty(DurableSlotTarget),
    BodyFlushed(DurableSlotTarget),
    FooterDirty(DurableSlotTarget),
}

pub struct AtaDurableStateBackend<'a, D> {
    device: D,
    binding: AtaDurableBinding,
    staging: &'a mut [u8; DURABLE_SLOT_BYTES],
    head: Option<AtaDurableHead>,
    phase: BackendPhase,
    flush_epoch: u64,
    last_device_error: Option<AtaPioError>,
}

impl<'a, D: AtaBlockDevice> AtaDurableStateBackend<'a, D> {
    pub fn new(
        device: D,
        binding: AtaDurableBinding,
        staging: &'a mut [u8; DURABLE_SLOT_BYTES],
    ) -> Result<Self, AtaDurableBackendInitError> {
        let identity = device
            .identity()
            .ok_or(AtaDurableBackendInitError::DeviceNotIdentified)?;
        if identity != binding.identity() {
            return Err(AtaDurableBackendInitError::DeviceIdentityMismatch {
                binding: binding.identity(),
                device: identity,
            });
        }
        Ok(Self {
            device,
            binding,
            staging,
            head: None,
            phase: BackendPhase::Idle,
            flush_epoch: 0,
            last_device_error: None,
        })
    }

    pub fn bind_head(&mut self, head: AtaDurableHead) -> Result<(), AtaDurableHeadBindError> {
        if self.head.is_some() {
            return Err(AtaDurableHeadBindError::AlreadyBound);
        }
        match head {
            AtaDurableHead::Recovered(0) => {
                return Err(AtaDurableHeadBindError::ZeroRecoveredGeneration)
            }
            AtaDurableHead::Recovered(u64::MAX) => {
                return Err(AtaDurableHeadBindError::GenerationExhausted)
            }
            AtaDurableHead::Genesis | AtaDurableHead::Recovered(_) => {}
        }
        self.head = Some(head);
        Ok(())
    }

    pub const fn binding(&self) -> AtaDurableBinding {
        self.binding
    }

    pub const fn head(&self) -> Option<AtaDurableHead> {
        self.head
    }

    pub const fn flush_epoch(&self) -> u64 {
        self.flush_epoch
    }

    pub const fn last_device_error(&self) -> Option<AtaPioError> {
        self.last_device_error
    }

    pub const fn device(&self) -> &D {
        &self.device
    }

    pub fn into_device(self) -> D {
        self.device
    }

    fn ensure_resource(&self, storage: ResourceId) -> Result<(), DurableStateBackendError> {
        if storage == self.binding.storage() {
            Ok(())
        } else {
            Err(DurableStateBackendError::ResourceMismatch)
        }
    }

    fn validate_next_target(
        &self,
        target: DurableSlotTarget,
    ) -> Result<(), DurableStateBackendError> {
        let expected = self
            .head
            .and_then(AtaDurableHead::next_generation)
            .ok_or(DurableStateBackendError::PhaseViolation)?;
        if target.generation() == expected {
            Ok(())
        } else {
            Err(DurableStateBackendError::ActiveGenerationConflict)
        }
    }

    fn write_staging_sector(
        &mut self,
        slot: DurableSlot,
        sector: u64,
    ) -> Result<(), DurableStateBackendError> {
        let start = sector as usize * ATA_SECTOR_BYTES;
        let mut bytes = [0_u8; ATA_SECTOR_BYTES];
        bytes.copy_from_slice(&self.staging[start..start + ATA_SECTOR_BYTES]);
        let lba = self.binding.slot_lba(slot) + sector;
        let result = self.device.write_sector(lba, &bytes);
        self.map_device_result(result)
    }

    fn write_staging_slot(&mut self, slot: DurableSlot) -> Result<(), DurableStateBackendError> {
        for sector in 0..ATA_DURABLE_SLOT_SECTORS {
            self.write_staging_sector(slot, sector)?;
        }
        Ok(())
    }

    fn read_device_sector(
        &mut self,
        slot: DurableSlot,
        sector: u64,
        output: &mut [u8; ATA_SECTOR_BYTES],
    ) -> Result<(), DurableStateBackendError> {
        let lba = self.binding.slot_lba(slot) + sector;
        let result = self.device.read_sector(lba, output);
        self.map_device_result(result)
    }

    fn flush_device(&mut self) -> Result<(), DurableStateBackendError> {
        let result = self.device.flush_cache();
        self.map_device_result(result)
    }

    fn map_device_result(
        &mut self,
        result: Result<(), AtaPioError>,
    ) -> Result<(), DurableStateBackendError> {
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                self.last_device_error = Some(error);
                Err(match error {
                    AtaPioError::BusyTimeout
                    | AtaPioError::DataRequestTimeout
                    | AtaPioError::CommandCompletionTimeout => {
                        DurableStateBackendError::Interrupted
                    }
                    _ => DurableStateBackendError::StorageFault,
                })
            }
        }
    }
}
