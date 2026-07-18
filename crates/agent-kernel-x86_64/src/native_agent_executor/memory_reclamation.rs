//! Atomic fault-time cleanup for live native Agent runtime memory.
//!
//! This bare-metal executor child preflights capability authority, semantic
//! records, private leaves, pool ownership, event capacity, and physical proof
//! words for every live binding. It then retires Resources through the facade,
//! removes leaves, clears frames, commits ledgers, and attaches bounded evidence.

use agent_kernel_core::{EventKind, ResourceKind, ResourceStatus};
use agent_kernel_x86_64::{
    runtime_frame_pool::RuntimeFrameRelease,
    runtime_page::RuntimePageRelease,
    runtime_reclamation::{
        RuntimeReclamationCandidate, RuntimeReclamationLog, RUNTIME_RECLAMATION_CAPACITY,
    },
    runtime_region::RuntimeRegionRelease,
};

use crate::{
    agent_cpu::FaultedAgentCpu,
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

pub(super) fn reclaim(
    booted: &mut X86BootedKernel,
    memory_pool: &mut RuntimeMemoryPool,
    mut cpu: FaultedAgentCpu,
) -> Option<(FaultedAgentCpu, usize)> {
    let context = cpu.context();
    let plan = cpu.runtime_reclamation_plan()?;
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
            &cpu,
            context.agent(),
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
                context.agent(),
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
            || event.agent != context.agent()
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
                cpu.deactivate_runtime_page_reclamation(private, release.frames)
                    && memory_pool.release(release.pool)
                    && cpu.commit_runtime_page_reclamation(private)
            }
            PrivateRelease::Region(private) => {
                cpu.deactivate_runtime_region_reclamation(private, release.frames)
                    && memory_pool.release(release.pool)
                    && cpu.commit_runtime_region_reclamation(private)
            }
        };
        if !private_released {
            return None;
        }
    }

    if !cpu.runtime_memory_is_clear()
        || !memory_pool.agent_is_clear(context.agent())
        || !cpu.attach_reclamation(plan, log)
    {
        return None;
    }
    Some((cpu, plan.len()))
}

fn prepare_one(
    booted: &X86BootedKernel,
    memory_pool: &RuntimeMemoryPool,
    cpu: &FaultedAgentCpu,
    agent: agent_kernel_core::AgentId,
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
            let release = cpu.prepare_runtime_page_reclamation(
                candidate.resource(),
                candidate.cell(),
                cell.value,
                frames,
            )?;
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
            let release = cpu.prepare_runtime_region_reclamation(
                candidate.resource(),
                candidate.cell(),
                cell.value,
                frames,
            )?;
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
