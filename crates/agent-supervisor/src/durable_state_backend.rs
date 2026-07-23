//! Deterministic in-memory implementation of the durable-state HAL.
//!
//! This Supervisor adapter models volatile writes, explicit flush barriers,
//! dual-slot generations, and injected power loss without touching kernel
//! internals. Production storage remains a machine-layer responsibility.

mod backend;

use agent_kernel_core::{DurableSlot, ResourceId};
use agent_kernel_hal::{DurableSlotRegion, DurableStateBackendError, DURABLE_SLOT_BYTES};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InMemoryDurableSlotPhase {
    Empty,
    Prepared,
    Body,
    Committed,
}

#[derive(Copy, Clone)]
struct SlotState {
    generation: u64,
    phase: InMemoryDurableSlotPhase,
    dirty: Option<DurableSlotRegion>,
    body_length: usize,
    flush_epoch: u64,
}

impl SlotState {
    const EMPTY: Self = Self {
        generation: 0,
        phase: InMemoryDurableSlotPhase::Empty,
        dirty: None,
        body_length: 0,
        flush_epoch: 0,
    };
}

pub struct InMemoryDurableStateBackend {
    storage: ResourceId,
    volatile_slots: [Box<[u8; DURABLE_SLOT_BYTES]>; 2],
    durable_slots: [Box<[u8; DURABLE_SLOT_BYTES]>; 2],
    volatile_state: [SlotState; 2],
    durable_state: [SlotState; 2],
    flush_epoch: u64,
    operation_count: u64,
    interrupt_at: Option<u64>,
}

impl InMemoryDurableStateBackend {
    pub fn new(storage: ResourceId) -> Option<Self> {
        if storage.raw() == 0 {
            return None;
        }
        Some(Self {
            storage,
            volatile_slots: [empty_slot(), empty_slot()],
            durable_slots: [empty_slot(), empty_slot()],
            volatile_state: [SlotState::EMPTY; 2],
            durable_state: [SlotState::EMPTY; 2],
            flush_epoch: 0,
            operation_count: 0,
            interrupt_at: None,
        })
    }

    pub fn inject_interrupt_after(&mut self, operations: u64) -> Option<()> {
        if operations == 0 || self.interrupt_at.is_some() {
            return None;
        }
        self.interrupt_at = Some(self.operation_count.checked_add(operations)?);
        Some(())
    }

    pub const fn operation_count(&self) -> u64 {
        self.operation_count
    }

    pub const fn flush_epoch(&self) -> u64 {
        self.flush_epoch
    }

    pub fn durable_phase(&self, slot: DurableSlot) -> InMemoryDurableSlotPhase {
        self.durable_state[slot_index(slot)].phase
    }

    pub fn durable_body_length(&self, slot: DurableSlot) -> usize {
        self.durable_state[slot_index(slot)].body_length
    }

    pub fn active_generation(&self) -> Option<u64> {
        self.active_slot_index()
            .map(|index| self.durable_state[index].generation)
    }

    pub fn simulate_power_loss(&mut self) {
        self.interrupt_at = None;
        for index in 0..self.volatile_slots.len() {
            self.volatile_slots[index].copy_from_slice(self.durable_slots[index].as_ref());
            self.volatile_state[index] = self.durable_state[index];
        }
    }

    fn ensure_resource(&self, storage: ResourceId) -> Result<(), DurableStateBackendError> {
        if storage == self.storage {
            Ok(())
        } else {
            Err(DurableStateBackendError::ResourceMismatch)
        }
    }

    fn begin_operation(&mut self) -> Result<(), DurableStateBackendError> {
        self.operation_count = self
            .operation_count
            .checked_add(1)
            .ok_or(DurableStateBackendError::StorageFault)?;
        if self.interrupt_at == Some(self.operation_count) {
            self.interrupt_at = None;
            self.simulate_power_loss();
            return Err(DurableStateBackendError::Interrupted);
        }
        Ok(())
    }

    fn active_slot_index(&self) -> Option<usize> {
        let mut active: Option<usize> = None;
        for (index, state) in self.durable_state.iter().enumerate() {
            if state.phase != InMemoryDurableSlotPhase::Committed {
                continue;
            }
            if active
                .map(|current| state.generation > self.durable_state[current].generation)
                .unwrap_or(true)
            {
                active = Some(index);
            }
        }
        active
    }
}

fn empty_slot() -> Box<[u8; DURABLE_SLOT_BYTES]> {
    Box::new([0; DURABLE_SLOT_BYTES])
}

const fn slot_index(slot: DurableSlot) -> usize {
    match slot {
        DurableSlot::A => 0,
        DurableSlot::B => 1,
    }
}

#[cfg(test)]
mod tests;
