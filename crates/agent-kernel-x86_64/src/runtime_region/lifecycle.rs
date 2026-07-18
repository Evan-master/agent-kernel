//! Reservation, mapping, cancellation, and release transitions for regions.
//!
//! This architecture-library child mutates the fixed virtual-slot ledger under
//! stale-safe transaction tokens. Page-table work remains in the bare-metal
//! Agent-memory layer.

use agent_kernel_core::{MemoryCellId, ResourceId};

use crate::runtime_frame_pool::MAX_RUNTIME_REGION_PAGES;

use super::{RegionState, RuntimeRegionLedger, RuntimeRegionRelease, RuntimeRegionReservation};

impl RuntimeRegionLedger {
    pub fn reserve(
        &mut self,
        resource: ResourceId,
        page_count: usize,
    ) -> Option<RuntimeRegionReservation> {
        if resource.raw() == 0
            || page_count == 0
            || page_count > MAX_RUNTIME_REGION_PAGES
            || self
                .regions
                .iter()
                .any(|state| matches!(state, RegionState::Reserved { .. }))
        {
            return None;
        }
        let entry = self
            .regions
            .iter()
            .position(|state| *state == RegionState::Available)?;
        let start_slot = self.first_fit(page_count)?;
        let generation = self.generation.checked_add(1)?;
        let transaction = self.next_transaction;
        self.next_transaction = transaction.checked_add(1)?;
        let reservation = RuntimeRegionReservation::new(
            resource,
            start_slot,
            page_count,
            generation,
            transaction,
        );
        self.regions[entry] = RegionState::Reserved {
            resource,
            start_slot: start_slot as u8,
            page_count: page_count as u8,
            generation,
            transaction,
        };
        Some(reservation)
    }

    pub fn cancel(&mut self, reservation: RuntimeRegionReservation) -> bool {
        let Some(entry) = self.reservation_entry(reservation) else {
            return false;
        };
        self.regions[entry] = RegionState::Available;
        true
    }

    pub fn commit_mapping(
        &mut self,
        reservation: RuntimeRegionReservation,
        cell: MemoryCellId,
    ) -> bool {
        let Some(entry) = self.reservation_entry(reservation) else {
            return false;
        };
        if cell.raw() == 0
            || self.regions.iter().any(
                |state| matches!(state, RegionState::Mapped { cell: actual, .. } if *actual == cell),
            )
        {
            return false;
        }
        self.regions[entry] = RegionState::Mapped {
            resource: reservation.resource(),
            cell,
            start_slot: reservation.start_slot() as u8,
            page_count: reservation.page_count() as u8,
            generation: reservation.generation(),
            transaction: reservation.transaction(),
        };
        self.generation = reservation.generation();
        true
    }

    pub fn prepare_release(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
    ) -> Option<RuntimeRegionRelease> {
        Some(self.binding(resource, cell)?.release())
    }

    pub fn commit_release(&mut self, release: RuntimeRegionRelease) -> bool {
        let Some(entry) = self.regions.iter().position(|state| {
            matches!(*state, RegionState::Mapped {
                resource,
                cell,
                start_slot,
                page_count,
                generation,
                transaction,
            } if resource == release.resource()
                && cell == release.cell()
                && usize::from(start_slot) == release.start_slot()
                && usize::from(page_count) == release.page_count()
                && generation == release.generation()
                && transaction == release.transaction())
        }) else {
            return false;
        };
        self.regions[entry] = RegionState::Available;
        true
    }
}
