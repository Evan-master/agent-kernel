//! CPU-owner adapters for the shared runtime-memory cleanup transaction.
//!
//! This executor child presents identical planning, preparation, leaf removal,
//! and ledger commits for captured fault and pending-call CPUs. Authorization,
//! semantic retirement, and pool mutation remain with the transaction caller.

use agent_kernel_core::{MemoryCellId, MemoryValue, ResourceId};
use agent_kernel_x86_64::{
    runtime_page::RuntimePageRelease, runtime_reclamation::RuntimeReclamationPlan,
    runtime_region::RuntimeRegionRelease,
};

use crate::{
    agent_cpu::{FaultedAgentCpu, PendingAgentCallCpu},
    agent_memory::RuntimePhysicalFrameSet,
};

pub(super) trait RuntimeReclamationOwner {
    fn plan(&self) -> Option<RuntimeReclamationPlan>;

    fn prepare_page(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimePageRelease>;

    fn deactivate_page(
        &mut self,
        release: RuntimePageRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool;

    fn commit_page(&mut self, release: RuntimePageRelease) -> bool;

    fn prepare_region(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimeRegionRelease>;

    fn deactivate_region(
        &mut self,
        release: RuntimeRegionRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool;

    fn commit_region(&mut self, release: RuntimeRegionRelease) -> bool;

    fn memory_is_clear(&self) -> bool;
}

impl RuntimeReclamationOwner for FaultedAgentCpu {
    fn plan(&self) -> Option<RuntimeReclamationPlan> {
        self.runtime_reclamation_plan()
    }

    fn prepare_page(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimePageRelease> {
        self.prepare_runtime_page_reclamation(resource, cell, descriptor, frames)
    }

    fn deactivate_page(
        &mut self,
        release: RuntimePageRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.deactivate_runtime_page_reclamation(release, frames)
    }

    fn commit_page(&mut self, release: RuntimePageRelease) -> bool {
        self.commit_runtime_page_reclamation(release)
    }

    fn prepare_region(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimeRegionRelease> {
        self.prepare_runtime_region_reclamation(resource, cell, descriptor, frames)
    }

    fn deactivate_region(
        &mut self,
        release: RuntimeRegionRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.deactivate_runtime_region_reclamation(release, frames)
    }

    fn commit_region(&mut self, release: RuntimeRegionRelease) -> bool {
        self.commit_runtime_region_reclamation(release)
    }

    fn memory_is_clear(&self) -> bool {
        self.runtime_memory_is_clear()
    }
}

impl RuntimeReclamationOwner for PendingAgentCallCpu {
    fn plan(&self) -> Option<RuntimeReclamationPlan> {
        self.runtime_reclamation_plan()
    }

    fn prepare_page(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimePageRelease> {
        self.prepare_runtime_page_release(resource, cell, descriptor, frames)
    }

    fn deactivate_page(
        &mut self,
        release: RuntimePageRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.deactivate_runtime_page(release, frames)
    }

    fn commit_page(&mut self, release: RuntimePageRelease) -> bool {
        self.commit_runtime_page_release(release)
    }

    fn prepare_region(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimeRegionRelease> {
        self.prepare_runtime_region_release(resource, cell, descriptor, frames)
    }

    fn deactivate_region(
        &mut self,
        release: RuntimeRegionRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.deactivate_runtime_region(release, frames)
    }

    fn commit_region(&mut self, release: RuntimeRegionRelease) -> bool {
        self.commit_runtime_region_release(release)
    }

    fn memory_is_clear(&self) -> bool {
        self.runtime_memory_is_clear()
    }
}
