//! Bounded kernel-selected execution loop for native ring-3 Agents.
//!
//! This bare-metal adapter combines physical dispatch ownership with public
//! semantic transitions. It never chooses an Agent, allocates memory, or
//! bypasses capability checks; the core run queue remains authoritative.

mod calls;
mod state;

use agent_kernel_core::{AgentId, CapabilityId, EventKind};
use agent_kernel_x86_64::native_runtime::NativeAgentRuntimeStore;

use crate::{
    agent_cpu::CompletedAgentCpu,
    native_agent_runtime::{NativeAgentContext, NativeAgentRuntime},
    X86BootedKernel,
};

const NATIVE_TASK_QUANTUM: u64 = 1;
const COMPLETED_AGENT_CAPACITY: usize = 3;

#[derive(Copy, Clone)]
pub(crate) struct NativeVerifyAuthority {
    agent: AgentId,
    capability: CapabilityId,
}

pub(crate) struct NativeExecutionReport {
    completed: NativeAgentRuntimeStore<CompletedAgentCpu, COMPLETED_AGENT_CAPACITY>,
}

#[derive(Copy, Clone, Default)]
pub(crate) struct NativeRuntimeEvidence {
    dispatches: u8,
    prepared: u8,
    preempted: u8,
    waiting: u8,
    yielded: u8,
}

pub(crate) fn run_until_idle(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
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
                let preempted = cpu.run_until_preempted()?;
                expire_quantum(booted, runtime, preempted)?;
            }
            NativeAgentContext::Preempted(cpu) => {
                evidence.preempted = evidence.preempted.checked_add(1)?;
                let pending = cpu.resume_until_agent_call()?;
                calls::run(booted, runtime, report, verify_authority, pending)?;
            }
            NativeAgentContext::WaitingCall(waiting) => {
                evidence.waiting = evidence.waiting.checked_add(1)?;
                let resumable = calls::resume_waiting_receive(booted, waiting)?;
                let pending = resumable.resume_until_agent_call()?;
                calls::run(booted, runtime, report, verify_authority, pending)?;
            }
            NativeAgentContext::YieldedCall(resumable) => {
                evidence.yielded = evidence.yielded.checked_add(1)?;
                let pending = resumable.resume_until_agent_call()?;
                calls::run(booted, runtime, report, verify_authority, pending)?;
            }
        }
        if entry.agent.raw() == 0 || entry.task.raw() == 0 {
            return None;
        }
    }
    Some(())
}

fn expire_quantum(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    cpu: crate::agent_cpu::PreemptedAgentCpu,
) -> Option<()> {
    let context = cpu.context();
    if cpu.tick_count() != 1 || !state::running(booted, context) {
        return None;
    }
    let event = booted
        .kernel_mut()
        .sys_tick_task(context.agent(), context.task())
        .ok()?;
    if event.kind != EventKind::TaskQuantumExpired
        || event.agent != context.agent()
        || event.task != Some(context.task())
        || event.task_ticks != Some(1)
        || event.task_quantum != Some(0)
        || !state::queued(booted, context)
        || runtime.park_preempted(cpu).is_some()
    {
        return None;
    }
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
        }
    }

    pub(super) fn record(&mut self, cpu: CompletedAgentCpu) -> Option<()> {
        let agent = cpu.context().agent();
        self.completed.insert(agent, cpu).ok()
    }

    pub(crate) fn completed(&self, agent: AgentId) -> Option<&CompletedAgentCpu> {
        self.completed.get(agent).ok()
    }

    pub(crate) const fn len(&self) -> usize {
        self.completed.len()
    }
}

impl NativeRuntimeEvidence {
    pub(crate) const fn proves_current_boot(self) -> bool {
        self.dispatches == 8
            && self.prepared == 3
            && self.preempted == 3
            && self.waiting == 1
            && self.yielded == 1
    }
}
