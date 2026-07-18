//! Faulted-CPU ownership transitions for runtime-memory reclamation.
//!
//! This CPU-layer child exposes bounded preparation and commit operations over
//! the captured Agent memory object. The native executor supplies validated
//! semantic records and physical frame sets; this module never authorizes or
//! retires Resources.

use agent_kernel_core::{MemoryCellId, MemoryValue, ResourceId};
use agent_kernel_x86_64::{
    runtime_page::RuntimePageRelease,
    runtime_reclamation::{RuntimeReclamationLog, RuntimeReclamationPlan},
    runtime_region::RuntimeRegionRelease,
};

use super::FaultedAgentCpu;
use crate::agent_memory::RuntimePhysicalFrameSet;

impl FaultedAgentCpu {
    pub(crate) fn runtime_reclamation_plan(&self) -> Option<RuntimeReclamationPlan> {
        self.memory.runtime_reclamation_plan()
    }

    pub(crate) fn prepare_runtime_page_reclamation(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimePageRelease> {
        self.memory
            .prepare_runtime_page_release(resource, cell, descriptor, frames)
    }

    pub(crate) fn deactivate_runtime_page_reclamation(
        &mut self,
        release: RuntimePageRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.memory.deactivate_runtime_page(release, frames)
    }

    pub(crate) fn commit_runtime_page_reclamation(&mut self, release: RuntimePageRelease) -> bool {
        self.memory.commit_runtime_page_release(release)
    }

    pub(crate) fn prepare_runtime_region_reclamation(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
        frames: RuntimePhysicalFrameSet,
    ) -> Option<RuntimeRegionRelease> {
        self.memory
            .prepare_runtime_region_release(resource, cell, descriptor, frames)
    }

    pub(crate) fn deactivate_runtime_region_reclamation(
        &mut self,
        release: RuntimeRegionRelease,
        frames: RuntimePhysicalFrameSet,
    ) -> bool {
        self.memory.deactivate_runtime_region(release, frames)
    }

    pub(crate) fn commit_runtime_region_reclamation(
        &mut self,
        release: RuntimeRegionRelease,
    ) -> bool {
        self.memory.commit_runtime_region_release(release)
    }

    pub(crate) fn attach_reclamation(
        &mut self,
        plan: RuntimeReclamationPlan,
        log: RuntimeReclamationLog,
    ) -> bool {
        if !self.reclamation.is_empty()
            || !log.matches_plan(plan)
            || !self.runtime_memory_is_clear()
        {
            return false;
        }
        self.reclamation = log;
        true
    }

    pub(crate) const fn reclamation_log(&self) -> RuntimeReclamationLog {
        self.reclamation
    }
}
