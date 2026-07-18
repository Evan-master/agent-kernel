//! Reversible pooled mapping for bounded multi-page Agent regions.
//!
//! This bare-metal memory child reserves contiguous virtual slots, validates
//! exact pooled frames and leaves, and retains first/last inspection evidence.
//! Resource, MemoryCell, and Event mutation remains in the executor.

use agent_kernel_core::{CapabilityId, MemoryCellId, MemoryValue, ResourceId};
use agent_kernel_x86_64::{
    runtime_region::{
        RuntimeRegionBinding, RuntimeRegionObservationLog, RuntimeRegionRelease,
        RuntimeRegionReservation, RUNTIME_MEMORY_ACCESS_READ_WRITE,
    },
    user_memory::PAGE_BYTES,
};

use super::{page_tables, PreparedAgentMemory, RuntimePhysicalFrameSet, PHYSICAL_MEMORY_OFFSET};

impl PreparedAgentMemory {
    pub(crate) fn prepare_runtime_region_allocation(
        &mut self,
        resource: ResourceId,
        capability: CapabilityId,
        page_count: usize,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<(RuntimeRegionReservation, MemoryValue)> {
        if !self.kernel_address_space_active() || frames.page_count() != page_count {
            return None;
        }
        let reservation = self
            .runtime_regions
            .reserve(resource, capability, page_count)?;
        if page_tables::activate_runtime_region(
            PHYSICAL_MEMORY_OFFSET,
            self.roots,
            self.layout,
            reservation.start_slot(),
            frames.as_slice(),
        )
        .is_none()
        {
            self.runtime_regions.cancel(reservation);
            return None;
        }
        Some((reservation, self.runtime_region_descriptor(reservation)))
    }

    pub(crate) fn commit_runtime_region_allocation(
        &mut self,
        reservation: RuntimeRegionReservation,
        cell: MemoryCellId,
    ) -> bool {
        self.runtime_regions.commit_mapping(reservation, cell)
    }

    pub(crate) fn rollback_runtime_region_allocation(
        &mut self,
        reservation: RuntimeRegionReservation,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        frames.page_count() == reservation.page_count()
            && page_tables::deactivate_runtime_region(
                PHYSICAL_MEMORY_OFFSET,
                self.roots,
                self.layout,
                reservation.start_slot(),
                frames.as_slice(),
            )
            .is_some()
            && self.runtime_regions.cancel(reservation)
            && self.runtime_region_is_absent(reservation.start_slot(), reservation.page_count())
    }

    pub(crate) fn validate_runtime_region(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimeRegionBinding> {
        let binding = self.runtime_regions.binding(resource, cell)?;
        (frames.page_count() == binding.page_count()
            && descriptor == self.runtime_region_binding_descriptor(binding)
            && page_tables::runtime_region_is_active(
                PHYSICAL_MEMORY_OFFSET,
                self.roots,
                self.layout,
                binding.start_slot(),
                frames.as_slice(),
            ))
        .then_some(binding)
    }

    pub(crate) fn can_record_runtime_region_observation(
        &self,
        binding: RuntimeRegionBinding,
    ) -> bool {
        self.runtime_region_observations.can_record(binding)
    }

    pub(crate) fn record_runtime_region_observation(
        &mut self,
        first: u64,
        last: u64,
        binding: RuntimeRegionBinding,
    ) -> bool {
        self.runtime_region_observations
            .record(binding, first, last)
    }

    pub(crate) fn prepare_runtime_region_release(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimeRegionRelease> {
        self.validate_runtime_region(resource, cell, descriptor, frames)?;
        self.runtime_regions.prepare_release(resource, cell)
    }

    pub(crate) fn deactivate_runtime_region(
        &mut self,
        release: RuntimeRegionRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.runtime_regions
            .binding(release.resource(), release.cell())
            .is_some_and(|binding| {
                binding.start_slot() == release.start_slot()
                    && binding.page_count() == release.page_count()
                    && binding.generation() == release.generation()
            })
            && frames.page_count() == release.page_count()
            && page_tables::deactivate_runtime_region(
                PHYSICAL_MEMORY_OFFSET,
                self.roots,
                self.layout,
                release.start_slot(),
                frames.as_slice(),
            )
            .is_some()
            && self.runtime_region_is_absent(release.start_slot(), release.page_count())
    }

    pub(crate) fn commit_runtime_region_release(&mut self, release: RuntimeRegionRelease) -> bool {
        self.runtime_regions.commit_release(release)
            && self
                .runtime_regions
                .binding(release.resource(), release.cell())
                .is_none()
            && self.runtime_region_is_absent(release.start_slot(), release.page_count())
    }

    pub(crate) fn runtime_region_generation(&self) -> u64 {
        self.runtime_regions.generation()
    }

    pub(crate) fn runtime_region_observations(&self) -> RuntimeRegionObservationLog {
        self.runtime_region_observations
    }

    pub(crate) fn runtime_regions_released(&self, generation: u64) -> bool {
        generation != 0
            && self.runtime_regions.generation() == generation
            && self.runtime_regions.is_clear()
            && self.runtime_region_is_absent(
                0,
                agent_kernel_x86_64::runtime_region::RUNTIME_REGION_SLOT_COUNT,
            )
    }

    pub(crate) fn runtime_memory_is_clear(&self) -> bool {
        self.runtime_page.is_available()
            && self.runtime_regions.is_clear()
            && self.runtime_page_is_absent()
            && self.runtime_region_is_absent(
                0,
                agent_kernel_x86_64::runtime_region::RUNTIME_REGION_SLOT_COUNT,
            )
    }

    fn runtime_region_descriptor(&self, reservation: RuntimeRegionReservation) -> MemoryValue {
        MemoryValue::new([
            self.layout
                .runtime_region_page_start(reservation.start_slot())
                .unwrap_or(0),
            PAGE_BYTES * reservation.page_count() as u64,
            RUNTIME_MEMORY_ACCESS_READ_WRITE,
            reservation.generation(),
        ])
    }

    fn runtime_region_binding_descriptor(&self, binding: RuntimeRegionBinding) -> MemoryValue {
        MemoryValue::new([
            self.layout
                .runtime_region_page_start(binding.start_slot())
                .unwrap_or(0),
            PAGE_BYTES * binding.page_count() as u64,
            RUNTIME_MEMORY_ACCESS_READ_WRITE,
            binding.generation(),
        ])
    }

    fn runtime_region_is_absent(&self, start_slot: usize, page_count: usize) -> bool {
        page_tables::runtime_region_is_absent(
            PHYSICAL_MEMORY_OFFSET,
            self.roots,
            self.layout,
            start_slot,
            page_count,
        )
    }
}
