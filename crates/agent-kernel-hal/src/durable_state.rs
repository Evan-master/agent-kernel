//! Fixed-capacity durable-state storage contract.
//!
//! This no_std HAL layer exposes semantic slot regions, flush barriers, and
//! full-slot readback without raw block offsets or allocation. Implementations
//! own device mechanics; higher layers own capsule encoding and verification.

mod backend;

pub use backend::{
    DurableFlush, DurableSlotReadback, DurableStateBackend, DurableStateBackendError,
};

use agent_kernel_core::{DurableSlot, ResourceId};

pub const DURABLE_SLOT_BYTES: usize = 64 * 1024;
pub const DURABLE_SLOT_HEADER_BYTES: usize = 64;
pub const DURABLE_SLOT_FOOTER_BYTES: usize = 64;
pub const DURABLE_SLOT_BODY_BYTES: usize =
    DURABLE_SLOT_BYTES - DURABLE_SLOT_HEADER_BYTES - DURABLE_SLOT_FOOTER_BYTES;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DurableSlotRegion {
    PreparedHeader = 1,
    Body = 2,
    CommitFooter = 3,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableSlotTargetError {
    ZeroStorageResource,
    ZeroGeneration,
    SlotGenerationMismatch {
        expected: DurableSlot,
        actual: DurableSlot,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableSlotTarget {
    storage: ResourceId,
    slot: DurableSlot,
    generation: u64,
}

impl DurableSlotTarget {
    pub fn new(
        storage: ResourceId,
        slot: DurableSlot,
        generation: u64,
    ) -> Result<Self, DurableSlotTargetError> {
        if storage.raw() == 0 {
            return Err(DurableSlotTargetError::ZeroStorageResource);
        }
        let expected = DurableSlot::for_generation(generation)
            .ok_or(DurableSlotTargetError::ZeroGeneration)?;
        if slot != expected {
            return Err(DurableSlotTargetError::SlotGenerationMismatch {
                expected,
                actual: slot,
            });
        }
        Ok(Self {
            storage,
            slot,
            generation,
        })
    }

    pub const fn storage(self) -> ResourceId {
        self.storage
    }

    pub const fn slot(self) -> DurableSlot {
        self.slot
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableSlotWriteError {
    HeaderLengthMismatch { length: usize, required: usize },
    BodyLengthOutOfRange { length: usize, limit: usize },
    FooterLengthMismatch { length: usize, required: usize },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableSlotWrite<'a> {
    target: DurableSlotTarget,
    region: DurableSlotRegion,
    bytes: &'a [u8],
}

impl<'a> DurableSlotWrite<'a> {
    pub fn new(
        target: DurableSlotTarget,
        region: DurableSlotRegion,
        bytes: &'a [u8],
    ) -> Result<Self, DurableSlotWriteError> {
        match region {
            DurableSlotRegion::PreparedHeader if bytes.len() != DURABLE_SLOT_HEADER_BYTES => {
                return Err(DurableSlotWriteError::HeaderLengthMismatch {
                    length: bytes.len(),
                    required: DURABLE_SLOT_HEADER_BYTES,
                });
            }
            DurableSlotRegion::Body
                if bytes.is_empty() || bytes.len() > DURABLE_SLOT_BODY_BYTES =>
            {
                return Err(DurableSlotWriteError::BodyLengthOutOfRange {
                    length: bytes.len(),
                    limit: DURABLE_SLOT_BODY_BYTES,
                });
            }
            DurableSlotRegion::CommitFooter if bytes.len() != DURABLE_SLOT_FOOTER_BYTES => {
                return Err(DurableSlotWriteError::FooterLengthMismatch {
                    length: bytes.len(),
                    required: DURABLE_SLOT_FOOTER_BYTES,
                });
            }
            _ => {}
        }
        Ok(Self {
            target,
            region,
            bytes,
        })
    }

    pub const fn target(self) -> DurableSlotTarget {
        self.target
    }

    pub const fn region(self) -> DurableSlotRegion {
        self.region
    }

    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }
}
