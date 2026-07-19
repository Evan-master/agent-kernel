//! Bounded kernel-selected execution loop for native ring-3 Agents.
//!
//! This bare-metal adapter combines physical dispatch ownership with public
//! semantic transitions. It never chooses an Agent, allocates memory, or
//! bypasses capability checks; the core run queue remains authoritative.

mod address_space_reclamation;
mod calls;
mod event_archive;
mod evidence;
mod memory_reclamation;
mod state;

use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, EventKind, FaultKind, MemoryCellId};
use agent_kernel_x86_64::native_runtime::NativeAgentRuntimeStore;

use crate::{
    agent_cpu::{AgentRunOutcome, CompletedAgentCpu, FaultedAgentCpu},
    agent_memory::RuntimeMemoryPool,
    native_agent_runtime::{NativeAgentContext, NativeAgentRuntime},
    X86BootedKernel,
};

pub(crate) use event_archive::{NativeEventArchive, NATIVE_EVENT_ARCHIVE_CAPACITY};

const NATIVE_TASK_QUANTUM: u64 = 1;
const COMPLETED_AGENT_CAPACITY: usize = 6;
const FAULTED_AGENT_CAPACITY: usize = 1;

#[derive(Copy, Clone)]
pub(crate) struct NativeVerifyAuthority {
    agent: AgentId,
    capability: CapabilityId,
}

pub(crate) struct NativeExecutionReport {
    completed: NativeAgentRuntimeStore<CompletedAgentCpu, COMPLETED_AGENT_CAPACITY>,
    faulted: NativeAgentRuntimeStore<FaultedAgentCpu, FAULTED_AGENT_CAPACITY>,
    event_archive: NativeEventArchive,
}

#[derive(Copy, Clone, Default)]
pub(crate) struct NativeRuntimeEvidence {
    dispatches: u8,
    prepared: u8,
    preempted: u8,
    waiting: u8,
    yielded: u8,
    recovered_faults: u8,
    quantum_expiries: u8,
    returning_quantum_expiries: u8,
    returning_quantum_generation: u8,
    agent_faults: u8,
}

pub(crate) fn run_until_idle(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &mut RuntimeMemoryPool,
    report: &mut NativeExecutionReport,
    evidence: &mut NativeRuntimeEvidence,
    verify_authority: Option<NativeVerifyAuthority>,
) -> Option<()> {
    while !booted.kernel().run_queue().is_empty() {
        let dispatched = runtime.dispatch_next(booted, NATIVE_TASK_QUANTUM)?;
        let entry = dispatched.entry();
        evidence.dispatches = evidence.dispatches.checked_add(1)?;
        match dispatched.into_context() {
            NativeAgentContext::Prepared(cpu) => {
                evidence.prepared = evidence.prepared.checked_add(1)?;
                run_outcome(
                    booted,
                    runtime,
                    memory_pool,
                    report,
                    evidence,
                    verify_authority,
                    cpu.run_until_boundary()?,
                )?;
            }
            NativeAgentContext::Preempted(cpu) => {
                evidence.preempted = evidence.preempted.checked_add(1)?;
                run_outcome(
                    booted,
                    runtime,
                    memory_pool,
                    report,
                    evidence,
                    verify_authority,
                    cpu.resume_until_boundary()?,
                )?;
            }
            NativeAgentContext::WaitingCall(waiting) => {
                evidence.waiting = evidence.waiting.checked_add(1)?;
                let resumable = calls::resume_waiting_receive(booted, waiting)?;
                run_outcome(
                    booted,
                    runtime,
                    memory_pool,
                    report,
                    evidence,
                    verify_authority,
                    resumable.resume_until_boundary()?,
                )?;
            }
            NativeAgentContext::YieldedCall(resumable) => {
                evidence.yielded = evidence.yielded.checked_add(1)?;
                run_outcome(
                    booted,
                    runtime,
                    memory_pool,
                    report,
                    evidence,
                    verify_authority,
                    resumable.resume_until_boundary()?,
                )?;
            }
            NativeAgentContext::RecoveredFault(resumable) => {
                evidence.recovered_faults = evidence.recovered_faults.checked_add(1)?;
                run_outcome(
                    booted,
                    runtime,
                    memory_pool,
                    report,
                    evidence,
                    verify_authority,
                    resumable.resume_until_boundary()?,
                )?;
            }
        }
        if entry.agent.raw() == 0 || entry.task.raw() == 0 {
            return None;
        }
    }
    Some(())
}

fn run_outcome(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &mut RuntimeMemoryPool,
    report: &mut NativeExecutionReport,
    evidence: &mut NativeRuntimeEvidence,
    verify_authority: Option<NativeVerifyAuthority>,
    outcome: AgentRunOutcome,
) -> Option<()> {
    match outcome {
        AgentRunOutcome::Call(pending) => calls::run(
            booted,
            runtime,
            memory_pool,
            report,
            evidence,
            verify_authority,
            pending,
        ),
        AgentRunOutcome::Preempted(cpu) => expire_quantum(booted, runtime, evidence, cpu),
        AgentRunOutcome::Fault(cpu) => contain_fault(booted, memory_pool, report, evidence, cpu),
    }
}

pub(super) fn contain_fault(
    booted: &mut X86BootedKernel,
    memory_pool: &mut RuntimeMemoryPool,
    report: &mut NativeExecutionReport,
    evidence: &mut NativeRuntimeEvidence,
    cpu: FaultedAgentCpu,
) -> Option<()> {
    let context = cpu.context();
    if !state::running(booted, context) {
        return None;
    }
    let (cpu, reclaimed) = memory_reclamation::reclaim(booted, memory_pool, cpu)?;
    let fault_kind = FaultKind::ExecutionTrap;
    let fault_detail = cpu.fault().detail();
    let agent_faults = evidence.agent_faults.checked_add(1)?;
    if !cpu.runtime_memory_is_clear() || !memory_pool.agent_is_clear(context.agent()) {
        return None;
    }
    let fault = booted
        .kernel_mut()
        .sys_fault_task(context.agent(), context.task(), fault_kind, fault_detail)
        .ok()?;
    let record = booted.kernel().faults().last()?;
    let event = booted.kernel().events().last()?;
    if record.id != fault
        || record.agent != context.agent()
        || record.task != context.task()
        || record.kind != fault_kind
        || record.detail != fault_detail
        || event.kind != EventKind::TaskFaulted
        || event.agent != context.agent()
        || event.task != Some(context.task())
        || event.fault != Some(fault)
        || event.fault_kind != Some(fault_kind)
        || event.fault_detail != Some(fault_detail)
        || !state::faulted(booted, context)
    {
        return None;
    }
    if reclaimed != 0 {
        crate::serial_write_line("AGENT_KERNEL_NATIVE_FAULT_MEMORY_RECLAIMED_OK");
    }
    report.record_faulted(cpu)?;
    evidence.agent_faults = agent_faults;
    Some(())
}

pub(super) fn expire_quantum(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    evidence: &mut NativeRuntimeEvidence,
    cpu: crate::agent_cpu::PreemptedAgentCpu,
) -> Option<()> {
    let context = cpu.context();
    let quantum_expiries = evidence.quantum_expiries.checked_add(1)?;
    let returning_quantum_expiries = if cpu.has_call_progress() {
        evidence.returning_quantum_expiries.checked_add(1)?
    } else {
        evidence.returning_quantum_expiries
    };
    let returning_quantum_generation = if cpu.has_call_progress() {
        cpu.physical_quantum_generation()
    } else {
        evidence.returning_quantum_generation
    };
    let (run_ticks, quantum_remaining) = state::running_progress(booted, context)?;
    let expected_ticks = run_ticks.checked_add(1)?;
    if cpu.tick_count() != 1 || quantum_remaining != 1 {
        return None;
    }
    let event = booted
        .kernel_mut()
        .sys_tick_task(context.agent(), context.task())
        .ok()?;
    if event.kind != EventKind::TaskQuantumExpired
        || event.agent != context.agent()
        || event.task != Some(context.task())
        || event.task_ticks != Some(expected_ticks)
        || event.task_quantum != Some(0)
        || !state::queued(booted, context)
        || runtime.park_preempted(cpu).is_some()
    {
        return None;
    }
    evidence.quantum_expiries = quantum_expiries;
    evidence.returning_quantum_expiries = returning_quantum_expiries;
    evidence.returning_quantum_generation = returning_quantum_generation;
    Some(())
}

impl NativeVerifyAuthority {
    pub(crate) const fn new(agent: AgentId, capability: CapabilityId) -> Option<Self> {
        if agent.raw() == 0 || capability.raw() == 0 {
            None
        } else {
            Some(Self { agent, capability })
        }
    }

    pub(super) const fn resolve(self, agent: AgentId) -> Option<CapabilityId> {
        if self.agent.raw() == agent.raw() {
            Some(self.capability)
        } else {
            None
        }
    }
}

impl NativeExecutionReport {
    pub(crate) fn new() -> Self {
        Self {
            completed: NativeAgentRuntimeStore::new(),
            faulted: NativeAgentRuntimeStore::new(),
            event_archive: NativeEventArchive::new(),
        }
    }

    pub(super) fn record(&mut self, cpu: CompletedAgentCpu) -> Option<()> {
        let agent = cpu.context().agent();
        self.completed.insert(agent, cpu).ok()
    }

    pub(crate) fn completed(&self, agent: AgentId) -> Option<&CompletedAgentCpu> {
        self.completed.get(agent).ok()
    }

    fn record_faulted(&mut self, cpu: FaultedAgentCpu) -> Option<()> {
        let agent = cpu.context().agent();
        self.faulted.insert(agent, cpu).ok()
    }

    pub(crate) fn faulted(&self, agent: AgentId) -> Option<&FaultedAgentCpu> {
        self.faulted.get(agent).ok()
    }

    pub(crate) fn take_faulted(&mut self, agent: AgentId) -> Option<FaultedAgentCpu> {
        self.faulted.take(agent).ok()
    }

    pub(super) fn contains_image(&self, image: AgentImageId) -> bool {
        self.completed.any(|cpu| cpu.context().image() == image)
            || self.faulted.any(|cpu| cpu.context().image() == image)
    }

    pub(super) fn contains_memory_cell(&self, cell: MemoryCellId) -> bool {
        self.completed.any(|cpu| cpu.references_memory_cell(cell))
            || self.faulted.any(|cpu| cpu.references_memory_cell(cell))
    }

    pub(crate) const fn len(&self) -> usize {
        self.completed.len()
    }

    pub(crate) const fn faulted_len(&self) -> usize {
        self.faulted.len()
    }
}
