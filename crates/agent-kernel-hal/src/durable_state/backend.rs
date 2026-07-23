//! Device-facing operations and fixed-width completion values.
//!
//! Implementations receive only validated semantic writes. Flush epochs and
//! full-slot readback let the verifier distinguish volatile writes from bytes
//! that the selected storage Resource claims are durable.

use agent_kernel_core::{DurableSlot, ResourceId};

use super::{DurableSlotTarget, DurableSlotWrite};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableStateBackendError {
    ResourceMismatch,
    ActiveGenerationConflict,
    PhaseViolation,
    BufferTooSmall { required: usize, available: usize },
    Interrupted,
    FlushEpochExhausted,
    StorageFault,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableFlush {
    target: DurableSlotTarget,
    epoch: u64,
}

impl DurableFlush {
    pub const fn new(target: DurableSlotTarget, epoch: u64) -> Option<Self> {
        if epoch == 0 {
            None
        } else {
            Some(Self { target, epoch })
        }
    }

    pub const fn target(self) -> DurableSlotTarget {
        self.target
    }

    pub const fn epoch(self) -> u64 {
        self.epoch
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableSlotReadback {
    storage: ResourceId,
    slot: DurableSlot,
    bytes_read: usize,
    flush_epoch: u64,
}

impl DurableSlotReadback {
    pub const fn new(
        storage: ResourceId,
        slot: DurableSlot,
        bytes_read: usize,
        flush_epoch: u64,
    ) -> Self {
        Self {
            storage,
            slot,
            bytes_read,
            flush_epoch,
        }
    }

    pub const fn storage(self) -> ResourceId {
        self.storage
    }

    pub const fn slot(self) -> DurableSlot {
        self.slot
    }

    pub const fn bytes_read(self) -> usize {
        self.bytes_read
    }

    pub const fn flush_epoch(self) -> u64 {
        self.flush_epoch
    }
}

pub trait DurableStateBackend {
    fn write(&mut self, request: DurableSlotWrite<'_>) -> Result<(), DurableStateBackendError>;

    fn flush(
        &mut self,
        target: DurableSlotTarget,
    ) -> Result<DurableFlush, DurableStateBackendError>;

    fn read_slot(
        &mut self,
        storage: ResourceId,
        slot: DurableSlot,
        output: &mut [u8],
    ) -> Result<DurableSlotReadback, DurableStateBackendError>;
}
