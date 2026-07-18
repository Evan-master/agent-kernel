//! Pooled physical-frame mapping for one compatibility runtime page.
//!
//! This bare-metal memory child coordinates the per-Agent page ledger with a
//! frame selected by the global pool. It owns reversible leaf transitions and
//! retained observation evidence; semantic commits remain in the executor.

use agent_kernel_core::{MemoryCellId, MemoryValue, ResourceId};
use agent_kernel_x86_64::{
    runtime_page::{RuntimePageRelease, RuntimePageReservation, RUNTIME_PAGE_ACCESS_READ_WRITE},
    user_memory::PAGE_BYTES,
};

use super::{page_tables, PreparedAgentMemory, RuntimePhysicalFrameSet, PHYSICAL_MEMORY_OFFSET};

impl PreparedAgentMemory {
    pub(crate) fn prepare_runtime_page_allocation(
        &mut self,
        resource: ResourceId,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<(RuntimePageReservation, MemoryValue)> {
        if !self.kernel_address_space_active()
            || frames.page_count() != 1
            || !self.runtime_page.is_available()
            || !self.runtime_page_is_absent()
        {
            return None;
        }
        let reservation = self.runtime_page.reserve(resource)?;
        if page_tables::activate_runtime_page(
            PHYSICAL_MEMORY_OFFSET,
            self.roots,
            self.layout,
            frames.as_slice()[0],
        )
        .is_none()
        {
            self.runtime_page.cancel(reservation);
            return None;
        }
        Some((
            reservation,
            self.runtime_page_descriptor(reservation.generation()),
        ))
    }

    pub(crate) fn commit_runtime_page_allocation(
        &mut self,
        reservation: RuntimePageReservation,
        cell: MemoryCellId,
    ) -> bool {
        self.runtime_page.commit_mapping(reservation, cell)
    }

    pub(crate) fn rollback_runtime_page_allocation(
        &mut self,
        reservation: RuntimePageReservation,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        frames.page_count() == 1
            && page_tables::deactivate_runtime_page(
                PHYSICAL_MEMORY_OFFSET,
                self.roots,
                self.layout,
                frames.as_slice()[0],
            )
            .is_some()
            && self.runtime_page.cancel(reservation)
            && self.runtime_page_is_absent()
    }

    pub(crate) fn validate_runtime_page(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<u64> {
        let binding = self.runtime_page.binding()?;
        (frames.page_count() == 1
            && binding.resource() == resource
            && binding.cell() == cell
            && descriptor == self.runtime_page_descriptor(binding.generation())
            && page_tables::runtime_page_is_active(
                PHYSICAL_MEMORY_OFFSET,
                self.roots,
                self.layout,
                frames.as_slice()[0],
            ))
        .then_some(binding.generation())
    }

    pub(crate) fn record_runtime_page_observation(&mut self, value: u64) {
        self.runtime_page_observation = Some(value);
    }

    pub(crate) fn prepare_runtime_page_release(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimePageRelease> {
        self.validate_runtime_page(resource, cell, descriptor, frames)?;
        self.runtime_page.prepare_release(resource, cell)
    }

    pub(crate) fn deactivate_runtime_page(
        &mut self,
        release: RuntimePageRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.runtime_page
            .matches(release.resource(), release.cell(), release.generation())
            && frames.page_count() == 1
            && page_tables::deactivate_runtime_page(
                PHYSICAL_MEMORY_OFFSET,
                self.roots,
                self.layout,
                frames.as_slice()[0],
            )
            .is_some()
            && self.runtime_page_is_absent()
    }

    pub(crate) fn commit_runtime_page_release(&mut self, release: RuntimePageRelease) -> bool {
        self.runtime_page.commit_release(release)
            && self.runtime_page_released(release.generation())
    }

    pub(crate) fn runtime_page_generation(&self) -> u64 {
        self.runtime_page.generation()
    }

    pub(crate) fn runtime_page_observation(&self) -> Option<u64> {
        self.runtime_page_observation
    }

    pub(crate) fn runtime_page_released(&self, generation: u64) -> bool {
        generation != 0
            && self.runtime_page.generation() == generation
            && self.runtime_page.is_available()
            && self.runtime_page_is_absent()
    }

    fn runtime_page_descriptor(&self, generation: u64) -> MemoryValue {
        MemoryValue::new([
            self.layout.runtime_page_start(),
            PAGE_BYTES,
            RUNTIME_PAGE_ACCESS_READ_WRITE,
            generation,
        ])
    }

    pub(super) fn runtime_page_is_absent(&self) -> bool {
        page_tables::runtime_page_is_absent(PHYSICAL_MEMORY_OFFSET, self.roots, self.layout)
    }
}
