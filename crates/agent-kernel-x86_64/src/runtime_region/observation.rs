//! Fixed-capacity inspection evidence for runtime memory regions.
//!
//! This architecture-library child records ordered first/last proof values
//! against committed region bindings. It remains copyable and allocation-free
//! so owned Agent memory can transfer exact evidence into a completed CPU.

use agent_kernel_core::MemoryCellId;

use crate::runtime_frame_pool::MAX_RUNTIME_REGION_PAGES;

use super::{RuntimeRegionBinding, RUNTIME_REGION_SLOT_COUNT};

pub const RUNTIME_REGION_OBSERVATION_CAPACITY: usize = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRegionObservation {
    cell: MemoryCellId,
    start_slot: u8,
    page_count: u8,
    generation: u64,
    first: u64,
    last: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRegionObservationLog {
    entries: [Option<RuntimeRegionObservation>; RUNTIME_REGION_OBSERVATION_CAPACITY],
    len: u8,
}

impl RuntimeRegionObservationLog {
    pub const fn new() -> Self {
        Self {
            entries: [None; RUNTIME_REGION_OBSERVATION_CAPACITY],
            len: 0,
        }
    }

    pub fn can_record(&self, binding: RuntimeRegionBinding) -> bool {
        let len = self.len();
        let valid_range = binding.page_count() != 0
            && binding.page_count() <= MAX_RUNTIME_REGION_PAGES
            && binding
                .start_slot()
                .checked_add(binding.page_count())
                .is_some_and(|end| end <= RUNTIME_REGION_SLOT_COUNT);
        let generation_is_next = len == 0
            || self.entries[len - 1].is_some_and(|entry| entry.generation < binding.generation());
        len < RUNTIME_REGION_OBSERVATION_CAPACITY
            && binding.cell().raw() != 0
            && binding.generation() != 0
            && valid_range
            && generation_is_next
            && !self.entries[..len]
                .iter()
                .flatten()
                .any(|entry| entry.cell == binding.cell())
    }

    pub fn record(&mut self, binding: RuntimeRegionBinding, first: u64, last: u64) -> bool {
        if !self.can_record(binding) {
            return false;
        }
        let len = self.len();
        self.entries[len] = Some(RuntimeRegionObservation {
            cell: binding.cell(),
            start_slot: binding.start_slot() as u8,
            page_count: binding.page_count() as u8,
            generation: binding.generation(),
            first,
            last,
        });
        self.len += 1;
        true
    }

    pub const fn len(&self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> Option<RuntimeRegionObservation> {
        self.entries.get(index).copied().flatten()
    }

    pub const fn entries(
        self,
    ) -> [Option<RuntimeRegionObservation>; RUNTIME_REGION_OBSERVATION_CAPACITY] {
        self.entries
    }
}

impl Default for RuntimeRegionObservationLog {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeRegionObservation {
    pub const fn cell(self) -> MemoryCellId {
        self.cell
    }

    pub const fn start_slot(self) -> usize {
        self.start_slot as usize
    }

    pub const fn page_count(self) -> usize {
        self.page_count as usize
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn first(self) -> u64 {
        self.first
    }

    pub const fn last(self) -> u64 {
        self.last
    }
}
