//! Fixed-capacity semantic, private-leaf, and physical reclamation transaction.
//!
//! This executor child preflights every live binding before mutation, retires
//! Resources through the public facade, then removes leaves and returns zeroed
//! frames. Lifecycle-specific Task mutation occurs after this transaction.

use agent_kernel_core::{AgentId, EventKind, ResourceKind, ResourceStatus};
use agent_kernel_x86_64::{
    runtime_frame_pool::RuntimeFrameRelease,
    runtime_page::RuntimePageRelease,
    runtime_reclamation::{
        RuntimeReclamationCandidate, RuntimeReclamationLog, RuntimeReclamationPlan,
        RUNTIME_RECLAMATION_CAPACITY,
    },
    runtime_region::RuntimeRegionRelease,
};

use super::owner::RuntimeReclamationOwner;
use crate::{
    agent_memory::{RuntimeMemoryPool, RuntimePhysicalFrameSet},
    X86BootedKernel,
};

#[derive(Copy, Clone)]
enum PrivateRelease {
    Page(RuntimePageRelease),
    Region(RuntimeRegionRelease),
}

#[derive(Copy, Clone)]
struct PreparedRelease {
    candidate: RuntimeReclamationCandidate,
    pool: RuntimeFrameRelease,
    frames: RuntimePhysicalFrameSet,
    private: PrivateRelease,
}

pub(super) struct ReclamationOutcome<Owner> {
    pub(super) owner: Owner,
    pub(super) plan: RuntimeReclamationPlan,
    pub(super) log: RuntimeReclamationLog,
}

pub(super) fn execute<Owner: RuntimeReclamationOwner>(
    booted: &mut X86BootedKernel,
    memory_pool: &mut RuntimeMemoryPool,
    mut owner: Owner,
    agent: AgentId,
) -> Option<ReclamationOutcome<Owner>> {
    let plan = owner.plan()?;
    let event_slots = plan.len().checked_add(1)?;
    if !booted.kernel().has_event_capacity(event_slots) {
        return None;
    }

    let mut prepared = [None; RUNTIME_RECLAMATION_CAPACITY];
    let mut log = RuntimeReclamationLog::new();
    for (index, slot) in prepared.iter_mut().enumerate().take(plan.len()) {
        let candidate = plan.get(index)?;
        *slot = Some(prepare_one(
            booted,
            memory_pool,
            &owner,
            agent,
            candidate,
            &mut log,
        )?);
    }
    if !log.matches_plan(plan) {
        return None;
    }

    for release in prepared.iter().flatten().copied() {
        let event_start = booted.kernel().events().len();
        let event = booted
            .kernel_mut()
            .sys_retire_resource(
                agent,
                release.candidate.capability(),
                release.candidate.resource(),
            )
            .ok()?;
        let resource = booted
            .kernel()
            .resources()
            .iter()
            .find(|record| record.id == release.candidate.resource())?;
        if booted.kernel().events().len() != event_start + 1
            || event != booted.kernel().events()[event_start]
            || event.kind != EventKind::ResourceRetired
            || event.agent != agent
            || event.resource != Some(release.candidate.resource())
            || event.capability != Some(release.candidate.capability())
            || resource.status != ResourceStatus::Retired
        {
            return None;
        }
    }

    for release in prepared.iter().flatten().copied() {
        let private_released = match release.private {
            PrivateRelease::Page(private) => {
                owner.deactivate_page(private, release.frames)
                    && memory_pool.release(release.pool)
                    && owner.commit_page(private)
            }
            PrivateRelease::Region(private) => {
                owner.deactivate_region(private, release.frames)
                    && memory_pool.release(release.pool)
                    && owner.commit_region(private)
            }
        };
        if !private_released {
            return None;
        }
    }

    if !owner.memory_is_clear() || !memory_pool.agent_is_clear(agent) {
        return None;
    }
    Some(ReclamationOutcome { owner, plan, log })
}

fn prepare_one<Owner: RuntimeReclamationOwner>(
    booted: &X86BootedKernel,
    memory_pool: &RuntimeMemoryPool,
    owner: &Owner,
    agent: AgentId,
    candidate: RuntimeReclamationCandidate,
    log: &mut RuntimeReclamationLog,
) -> Option<PreparedRelease> {
    booted
        .kernel()
        .can_retire_resource(agent, candidate.capability(), candidate.resource())
        .ok()?;
    let resource = booted
        .kernel()
        .resources()
        .iter()
        .find(|record| record.id == candidate.resource())?;
    let cell = booted
        .kernel()
        .memory_cells()
        .iter()
        .find(|record| record.id == candidate.cell())?;
    if resource.kind != ResourceKind::Memory
        || resource.status != ResourceStatus::Active
        || cell.resource != candidate.resource()
        || cell.creator != agent
        || cell.last_writer != agent
        || cell.revision != 1
        || cell.value.words[3] != candidate.generation()
    {
        return None;
    }

    let pool_binding = memory_pool.binding(
        agent,
        candidate.resource(),
        candidate.cell(),
        candidate.generation(),
    )?;
    let pool = memory_pool.prepare_release(
        agent,
        candidate.resource(),
        candidate.cell(),
        candidate.generation(),
    )?;
    let frames = memory_pool.frame_set_for_release(pool)?;
    if pool_binding.page_count() != candidate.page_count()
        || pool.page_count() != candidate.page_count()
        || pool.agent() != agent
        || pool.resource() != candidate.resource()
        || pool.cell() != candidate.cell()
        || pool.generation() != candidate.generation()
    {
        return None;
    }

    let private = match candidate {
        RuntimeReclamationCandidate::Page(_) => {
            let release =
                owner.prepare_page(candidate.resource(), candidate.cell(), cell.value, frames)?;
            if release.resource() != candidate.resource()
                || release.capability() != candidate.capability()
                || release.cell() != candidate.cell()
                || release.generation() != candidate.generation()
            {
                return None;
            }
            PrivateRelease::Page(release)
        }
        RuntimeReclamationCandidate::Region(_) => {
            let release =
                owner.prepare_region(candidate.resource(), candidate.cell(), cell.value, frames)?;
            if release.resource() != candidate.resource()
                || release.capability() != candidate.capability()
                || release.cell() != candidate.cell()
                || release.page_count() != candidate.page_count()
                || release.generation() != candidate.generation()
            {
                return None;
            }
            PrivateRelease::Region(release)
        }
    };

    let (first, last) = memory_pool.observe(pool_binding)?;
    log.record(candidate, first, last)
        .then_some(PreparedRelease {
            candidate,
            pool,
            frames,
            private,
        })
}
