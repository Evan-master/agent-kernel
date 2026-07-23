//! HAL operations for the Supervisor in-memory durable-state backend.
//!
//! Region transitions require the preceding write to be flushed. A flush
//! copies one complete volatile slot and its metadata into the durable image in
//! one deterministic step; readback observes the durable image only.

use agent_kernel_core::{DurableSlot, ResourceId};
use agent_kernel_hal::{
    DurableFlush, DurableSlotReadback, DurableSlotRegion, DurableSlotTarget, DurableSlotWrite,
    DurableStateBackend, DurableStateBackendError, DURABLE_SLOT_BODY_BYTES, DURABLE_SLOT_BYTES,
    DURABLE_SLOT_FOOTER_BYTES, DURABLE_SLOT_HEADER_BYTES,
};

use super::{slot_index, InMemoryDurableSlotPhase, InMemoryDurableStateBackend, SlotState};

impl DurableStateBackend for InMemoryDurableStateBackend {
    fn write(&mut self, request: DurableSlotWrite<'_>) -> Result<(), DurableStateBackendError> {
        let target = request.target();
        self.ensure_resource(target.storage())?;
        self.begin_operation()?;
        let index = slot_index(target.slot());

        match request.region() {
            DurableSlotRegion::PreparedHeader => {
                if self.active_slot_index() == Some(index) {
                    return Err(DurableStateBackendError::ActiveGenerationConflict);
                }
                self.volatile_slots[index].fill(0);
                self.volatile_slots[index][..DURABLE_SLOT_HEADER_BYTES]
                    .copy_from_slice(request.bytes());
                self.volatile_state[index] = SlotState {
                    generation: target.generation(),
                    phase: InMemoryDurableSlotPhase::Prepared,
                    dirty: Some(DurableSlotRegion::PreparedHeader),
                    body_length: 0,
                    flush_epoch: self.durable_state[index].flush_epoch,
                };
            }
            DurableSlotRegion::Body => {
                let state = self.volatile_state[index];
                if state.generation != target.generation()
                    || state.phase != InMemoryDurableSlotPhase::Prepared
                    || state.dirty.is_some()
                {
                    return Err(DurableStateBackendError::PhaseViolation);
                }
                let body_start = DURABLE_SLOT_HEADER_BYTES;
                let body_end = body_start + DURABLE_SLOT_BODY_BYTES;
                self.volatile_slots[index][body_start..body_end].fill(0);
                let write_end = body_start + request.bytes().len();
                self.volatile_slots[index][body_start..write_end].copy_from_slice(request.bytes());
                self.volatile_state[index] = SlotState {
                    phase: InMemoryDurableSlotPhase::Body,
                    dirty: Some(DurableSlotRegion::Body),
                    body_length: request.bytes().len(),
                    ..state
                };
            }
            DurableSlotRegion::CommitFooter => {
                let state = self.volatile_state[index];
                if state.generation != target.generation()
                    || state.phase != InMemoryDurableSlotPhase::Body
                    || state.dirty.is_some()
                {
                    return Err(DurableStateBackendError::PhaseViolation);
                }
                let footer_start = DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES;
                self.volatile_slots[index][footer_start..].copy_from_slice(request.bytes());
                self.volatile_state[index] = SlotState {
                    phase: InMemoryDurableSlotPhase::Committed,
                    dirty: Some(DurableSlotRegion::CommitFooter),
                    ..state
                };
            }
        }
        Ok(())
    }

    fn flush(
        &mut self,
        target: DurableSlotTarget,
    ) -> Result<DurableFlush, DurableStateBackendError> {
        self.ensure_resource(target.storage())?;
        self.begin_operation()?;
        let index = slot_index(target.slot());
        let state = self.volatile_state[index];
        if state.generation != target.generation() || state.dirty.is_none() {
            return Err(DurableStateBackendError::PhaseViolation);
        }
        let epoch = self
            .flush_epoch
            .checked_add(1)
            .ok_or(DurableStateBackendError::FlushEpochExhausted)?;
        let durable_state = SlotState {
            dirty: None,
            flush_epoch: epoch,
            ..state
        };
        self.durable_slots[index].copy_from_slice(self.volatile_slots[index].as_ref());
        self.durable_state[index] = durable_state;
        self.volatile_state[index] = durable_state;
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
        self.begin_operation()?;
        let index = slot_index(slot);
        output[..DURABLE_SLOT_BYTES].copy_from_slice(self.durable_slots[index].as_ref());
        Ok(DurableSlotReadback::new(
            storage,
            slot,
            DURABLE_SLOT_BYTES,
            self.durable_state[index].flush_epoch,
        ))
    }
}
