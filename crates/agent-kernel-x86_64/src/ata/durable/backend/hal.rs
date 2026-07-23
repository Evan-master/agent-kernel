//! HAL operation sequencing for the ATA durable-state backend.

use agent_kernel_core::{DurableSlot, ResourceId};
use agent_kernel_hal::{
    DurableFlush, DurableSlotReadback, DurableSlotRegion, DurableSlotTarget, DurableSlotWrite,
    DurableStateBackend, DurableStateBackendError, DURABLE_SLOT_BODY_BYTES, DURABLE_SLOT_BYTES,
    DURABLE_SLOT_FOOTER_BYTES, DURABLE_SLOT_HEADER_BYTES,
};

use super::{AtaDurableStateBackend, BackendPhase};
use crate::ata::{AtaBlockDevice, AtaDurableHead, ATA_DURABLE_SLOT_SECTORS, ATA_SECTOR_BYTES};

impl<D: AtaBlockDevice> DurableStateBackend for AtaDurableStateBackend<'_, D> {
    fn write(&mut self, request: DurableSlotWrite<'_>) -> Result<(), DurableStateBackendError> {
        let target = request.target();
        self.ensure_resource(target.storage())?;
        self.validate_next_target(target)?;
        self.last_device_error = None;

        match request.region() {
            DurableSlotRegion::PreparedHeader => self.write_header(target, request.bytes()),
            DurableSlotRegion::Body => self.write_body(target, request.bytes()),
            DurableSlotRegion::CommitFooter => self.write_footer(target, request.bytes()),
        }
    }

    fn flush(
        &mut self,
        target: DurableSlotTarget,
    ) -> Result<DurableFlush, DurableStateBackendError> {
        self.ensure_resource(target.storage())?;
        self.validate_next_target(target)?;
        let next_phase = match self.phase {
            BackendPhase::HeaderDirty(actual) if actual == target => {
                BackendPhase::HeaderFlushed(target)
            }
            BackendPhase::BodyDirty(actual) if actual == target => {
                BackendPhase::BodyFlushed(target)
            }
            BackendPhase::FooterDirty(actual) if actual == target => BackendPhase::Idle,
            _ => return Err(DurableStateBackendError::PhaseViolation),
        };
        let epoch = self
            .flush_epoch
            .checked_add(1)
            .ok_or(DurableStateBackendError::FlushEpochExhausted)?;
        self.last_device_error = None;
        self.flush_device()?;

        if matches!(self.phase, BackendPhase::FooterDirty(_)) {
            self.head = Some(AtaDurableHead::Recovered(target.generation()));
        }
        self.phase = next_phase;
        self.flush_epoch = epoch;
        DurableFlush::new(target, epoch).ok_or(DurableStateBackendError::FlushEpochExhausted)
    }

    fn read_slot(
        &mut self,
        storage: ResourceId,
        slot: DurableSlot,
        output: &mut [u8],
    ) -> Result<DurableSlotReadback, DurableStateBackendError> {
        self.ensure_resource(storage)?;
        if output.len() < DURABLE_SLOT_BYTES {
            return Err(DurableStateBackendError::BufferTooSmall {
                required: DURABLE_SLOT_BYTES,
                available: output.len(),
            });
        }
        self.last_device_error = None;
        let mut sector_bytes = [0_u8; ATA_SECTOR_BYTES];
        for sector in 0..ATA_DURABLE_SLOT_SECTORS {
            self.read_device_sector(slot, sector, &mut sector_bytes)?;
            let start = sector as usize * ATA_SECTOR_BYTES;
            output[start..start + ATA_SECTOR_BYTES].copy_from_slice(&sector_bytes);
        }
        Ok(DurableSlotReadback::new(
            storage,
            slot,
            DURABLE_SLOT_BYTES,
            self.observed_flush_epoch(),
        ))
    }
}

impl<D: AtaBlockDevice> AtaDurableStateBackend<'_, D> {
    fn observed_flush_epoch(&self) -> u64 {
        if self.flush_epoch == 0 {
            1
        } else {
            self.flush_epoch
        }
    }

    fn write_header(
        &mut self,
        target: DurableSlotTarget,
        bytes: &[u8],
    ) -> Result<(), DurableStateBackendError> {
        if self.phase != BackendPhase::Idle {
            return Err(DurableStateBackendError::PhaseViolation);
        }
        self.staging.fill(0);
        self.staging[..DURABLE_SLOT_HEADER_BYTES].copy_from_slice(bytes);
        self.write_staging_sector(target.slot(), 0)?;
        self.write_staging_sector(target.slot(), ATA_DURABLE_SLOT_SECTORS - 1)?;
        self.phase = BackendPhase::HeaderDirty(target);
        Ok(())
    }

    fn write_body(
        &mut self,
        target: DurableSlotTarget,
        bytes: &[u8],
    ) -> Result<(), DurableStateBackendError> {
        if self.phase != BackendPhase::HeaderFlushed(target) {
            return Err(DurableStateBackendError::PhaseViolation);
        }
        let body_start = DURABLE_SLOT_HEADER_BYTES;
        let body_end = body_start + DURABLE_SLOT_BODY_BYTES;
        self.staging[body_start..body_end].fill(0);
        self.staging[body_start..body_start + bytes.len()].copy_from_slice(bytes);
        self.write_staging_slot(target.slot())?;
        self.phase = BackendPhase::BodyDirty(target);
        Ok(())
    }

    fn write_footer(
        &mut self,
        target: DurableSlotTarget,
        bytes: &[u8],
    ) -> Result<(), DurableStateBackendError> {
        if self.phase != BackendPhase::BodyFlushed(target) {
            return Err(DurableStateBackendError::PhaseViolation);
        }
        let footer_start = DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES;
        self.staging[footer_start..].copy_from_slice(bytes);
        self.write_staging_sector(target.slot(), ATA_DURABLE_SLOT_SECTORS - 1)?;
        self.phase = BackendPhase::FooterDirty(target);
        Ok(())
    }
}
