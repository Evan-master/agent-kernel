//! Shared fault-time and completion-time runtime-memory reclamation.
//!
//! This executor module owns lifecycle entry points around one fixed-capacity
//! cleanup transaction. CPU-specific memory access lives in `owner`; semantic,
//! private-leaf, and physical-pool commit order lives in `transaction`.

mod owner;
mod transaction;

use agent_kernel_x86_64::runtime_reclamation::RuntimeReclamationLog;

use crate::{
    agent_cpu::{FaultedAgentCpu, PendingAgentCallCpu},
    agent_memory::RuntimeMemoryPool,
    X86BootedKernel,
};

pub(super) fn reclaim(
    booted: &mut X86BootedKernel,
    memory_pool: &mut RuntimeMemoryPool,
    cpu: FaultedAgentCpu,
) -> Option<(FaultedAgentCpu, usize)> {
    let agent = cpu.context().agent();
    let outcome = transaction::execute(booted, memory_pool, cpu, agent)?;
    let reclaimed = outcome.plan.len();
    let mut cpu = outcome.owner;
    cpu.attach_reclamation(outcome.plan, outcome.log)
        .then_some((cpu, reclaimed))
}

pub(super) fn reclaim_completion(
    booted: &mut X86BootedKernel,
    memory_pool: &mut RuntimeMemoryPool,
    cpu: PendingAgentCallCpu,
) -> Option<(PendingAgentCallCpu, RuntimeReclamationLog, usize)> {
    let agent = cpu.context().agent();
    let outcome = transaction::execute(booted, memory_pool, cpu, agent)?;
    Some((outcome.owner, outcome.log, outcome.plan.len()))
}
