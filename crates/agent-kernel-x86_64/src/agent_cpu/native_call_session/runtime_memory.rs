//! CPU-session access to native runtime-memory transactions.
//!
//! These role-independent wrappers retain the owned Agent memory object while
//! the executor coordinates semantic records and the global physical pool.

use agent_kernel_core::{CapabilityId, MemoryCellId, MemoryValue, ResourceId};
use agent_kernel_x86_64::{
    runtime_page::{RuntimePageRelease, RuntimePageReservation},
    runtime_reclamation::RuntimeReclamationPlan,
    runtime_region::{RuntimeRegionBinding, RuntimeRegionRelease, RuntimeRegionReservation},
};

use super::{CompletedAgentCpu, PendingAgentCallCpu, ResumableAgentCpu, WaitingAgentCallCpu};
use crate::agent_memory::RuntimePhysicalFrameSet;

impl PendingAgentCallCpu {
    pub(crate) fn references_memory_cell(&self, cell: MemoryCellId) -> bool {
        self.session.memory.references_memory_cell(cell)
    }

    pub(crate) fn runtime_reclamation_plan(&self) -> Option<RuntimeReclamationPlan> {
        self.session.memory.runtime_reclamation_plan()
    }

    pub(crate) fn prepare_runtime_page_allocation(
        &mut self,
        resource: ResourceId,
        capability: CapabilityId,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<(RuntimePageReservation, MemoryValue)> {
        self.session
            .memory
            .prepare_runtime_page_allocation(resource, capability, frames)
    }

    pub(crate) fn commit_runtime_page_allocation(
        &mut self,
        reservation: RuntimePageReservation,
        cell: MemoryCellId,
    ) -> bool {
        self.session
            .memory
            .commit_runtime_page_allocation(reservation, cell)
    }

    pub(crate) fn rollback_runtime_page_allocation(
        &mut self,
        reservation: RuntimePageReservation,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.session
            .memory
            .rollback_runtime_page_allocation(reservation, frames)
    }

    pub(crate) fn validate_runtime_page(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<u64> {
        self.session
            .memory
            .validate_runtime_page(resource, cell, descriptor, frames)
    }

    pub(crate) fn record_runtime_page_observation(&mut self, value: u64) {
        self.session.memory.record_runtime_page_observation(value);
    }

    pub(crate) fn prepare_runtime_page_release(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimePageRelease> {
        self.session
            .memory
            .prepare_runtime_page_release(resource, cell, descriptor, frames)
    }

    pub(crate) fn deactivate_runtime_page(
        &mut self,
        release: RuntimePageRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.session.memory.deactivate_runtime_page(release, frames)
    }

    pub(crate) fn commit_runtime_page_release(&mut self, release: RuntimePageRelease) -> bool {
        self.session.memory.commit_runtime_page_release(release)
    }

    pub(crate) fn prepare_runtime_region_allocation(
        &mut self,
        resource: ResourceId,
        capability: CapabilityId,
        page_count: usize,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<(RuntimeRegionReservation, MemoryValue)> {
        self.session
            .memory
            .prepare_runtime_region_allocation(resource, capability, page_count, frames)
    }

    pub(crate) fn commit_runtime_region_allocation(
        &mut self,
        reservation: RuntimeRegionReservation,
        cell: MemoryCellId,
    ) -> bool {
        self.session
            .memory
            .commit_runtime_region_allocation(reservation, cell)
    }

    pub(crate) fn rollback_runtime_region_allocation(
        &mut self,
        reservation: RuntimeRegionReservation,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.session
            .memory
            .rollback_runtime_region_allocation(reservation, frames)
    }

    pub(crate) fn validate_runtime_region(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimeRegionBinding> {
        self.session
            .memory
            .validate_runtime_region(resource, cell, descriptor, frames)
    }

    pub(crate) fn record_runtime_region_observation(
        &mut self,
        first: u64,
        last: u64,
        binding: RuntimeRegionBinding,
    ) -> bool {
        self.session
            .memory
            .record_runtime_region_observation(first, last, binding)
    }

    pub(crate) fn can_record_runtime_region_observation(
        &self,
        binding: RuntimeRegionBinding,
    ) -> bool {
        self.session
            .memory
            .can_record_runtime_region_observation(binding)
    }

    pub(crate) fn prepare_runtime_region_release(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimeRegionRelease> {
        self.session
            .memory
            .prepare_runtime_region_release(resource, cell, descriptor, frames)
    }

    pub(crate) fn deactivate_runtime_region(
        &mut self,
        release: RuntimeRegionRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.session
            .memory
            .deactivate_runtime_region(release, frames)
    }

    pub(crate) fn commit_runtime_region_release(&mut self, release: RuntimeRegionRelease) -> bool {
        self.session.memory.commit_runtime_region_release(release)
    }

    pub(crate) fn runtime_memory_is_clear(&self) -> bool {
        self.session.memory.runtime_memory_is_clear()
    }
}

impl ResumableAgentCpu {
    pub(crate) fn references_memory_cell(&self, cell: MemoryCellId) -> bool {
        self.0.memory.references_memory_cell(cell)
    }
}

impl WaitingAgentCallCpu {
    pub(crate) fn references_memory_cell(&self, cell: MemoryCellId) -> bool {
        self.pending.references_memory_cell(cell)
    }
}

impl CompletedAgentCpu {
    pub(crate) fn references_memory_cell(&self, cell: MemoryCellId) -> bool {
        self.memory.references_memory_cell(cell)
    }
}
