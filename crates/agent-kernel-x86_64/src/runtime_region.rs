//! Pure virtual ownership ledger for bounded x86 runtime memory regions.
//!
//! This architecture-library module assigns contiguous slots inside one
//! Agent address space and binds them to Resource, MemoryCell, generation, and
//! stale-safe transaction identity. Page tables remain in the bare-metal layer.

mod lifecycle;
mod observation;
mod types;

use agent_kernel_core::{CapabilityId, MemoryCellId, ResourceId};

pub use observation::{
    RuntimeRegionObservation, RuntimeRegionObservationLog, RUNTIME_REGION_OBSERVATION_CAPACITY,
};
pub use types::{RuntimeRegionBinding, RuntimeRegionRelease, RuntimeRegionReservation};

pub const RUNTIME_REGION_SLOT_COUNT: usize = 8;
pub const RUNTIME_REGION_CAPACITY: usize = 4;
pub const RUNTIME_MEMORY_ACCESS_READ_WRITE: u64 = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RegionState {
    Available,
    Reserved {
        resource: ResourceId,
        capability: CapabilityId,
        start_slot: u8,
        page_count: u8,
        generation: u64,
        transaction: u64,
    },
    Mapped {
        resource: ResourceId,
        capability: CapabilityId,
        cell: MemoryCellId,
        start_slot: u8,
        page_count: u8,
        generation: u64,
        transaction: u64,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRegionLedger {
    regions: [RegionState; RUNTIME_REGION_CAPACITY],
    generation: u64,
    next_transaction: u64,
}

impl RuntimeRegionLedger {
    pub const fn new() -> Self {
        Self {
            regions: [RegionState::Available; RUNTIME_REGION_CAPACITY],
            generation: 0,
            next_transaction: 1,
        }
    }

    pub fn binding(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
    ) -> Option<RuntimeRegionBinding> {
        self.regions.iter().find_map(|state| match *state {
            RegionState::Mapped {
                resource: actual_resource,
                capability,
                cell: actual_cell,
                start_slot,
                page_count,
                generation,
                transaction,
            } if actual_resource == resource && actual_cell == cell => {
                Some(RuntimeRegionBinding::new(
                    resource,
                    capability,
                    cell,
                    usize::from(start_slot),
                    usize::from(page_count),
                    generation,
                    transaction,
                ))
            }
            _ => None,
        })
    }

    pub fn active_region_count(&self) -> usize {
        self.regions
            .iter()
            .filter(|state| matches!(state, RegionState::Mapped { .. }))
            .count()
    }

    pub fn bindings(&self) -> [Option<RuntimeRegionBinding>; RUNTIME_REGION_CAPACITY] {
        self.regions.map(|state| match state {
            RegionState::Mapped {
                resource,
                capability,
                cell,
                start_slot,
                page_count,
                generation,
                transaction,
            } => Some(RuntimeRegionBinding::new(
                resource,
                capability,
                cell,
                usize::from(start_slot),
                usize::from(page_count),
                generation,
                transaction,
            )),
            _ => None,
        })
    }

    pub fn is_clear(&self) -> bool {
        self.regions
            .iter()
            .all(|state| *state == RegionState::Available)
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    fn reservation_entry(&self, reservation: RuntimeRegionReservation) -> Option<usize> {
        self.regions.iter().position(|state| {
            matches!(*state, RegionState::Reserved {
                resource,
                capability,
                start_slot,
                page_count,
                generation,
                transaction,
            } if resource == reservation.resource()
                && capability == reservation.capability()
                && usize::from(start_slot) == reservation.start_slot()
                && usize::from(page_count) == reservation.page_count()
                && generation == reservation.generation()
                && transaction == reservation.transaction())
        })
    }

    fn first_fit(&self, page_count: usize) -> Option<usize> {
        (0..=RUNTIME_REGION_SLOT_COUNT.checked_sub(page_count)?).find(|start| {
            self.regions.iter().all(|state| match *state {
                RegionState::Available => true,
                RegionState::Reserved {
                    start_slot,
                    page_count: occupied_count,
                    ..
                }
                | RegionState::Mapped {
                    start_slot,
                    page_count: occupied_count,
                    ..
                } => {
                    let occupied_start = usize::from(start_slot);
                    let occupied_end = occupied_start + usize::from(occupied_count);
                    *start + page_count <= occupied_start || *start >= occupied_end
                }
            })
        })
    }
}

impl Default for RuntimeRegionLedger {
    fn default() -> Self {
        Self::new()
    }
}
