//! Two-Worker semantic schedule for physical x86 context rotation.
//!
//! This boot adapter owns only type-state ordering between architecture evidence
//! and public task syscalls. Setup admits both Workers; transition helpers prove
//! FIFO queue state across two expiries and two cooperative yields.

mod setup;
mod transitions;

use agent_kernel_core::{AgentId, TaskId};

use crate::{
    agent_cpu::{PreemptedAgentCpu, YieldedAgentCpu},
    X86BootedKernel,
};

pub(super) const WORKER_A: AgentId = AgentId::new(3);
pub(super) const WORKER_B: AgentId = AgentId::new(4);
pub(super) const TASK_QUANTUM: u64 = 1;

#[derive(Copy, Clone)]
pub(super) struct WorkerTask {
    agent: AgentId,
    task: TaskId,
}

impl WorkerTask {
    const fn new(agent: AgentId, task: TaskId) -> Self {
        Self { agent, task }
    }
}

pub(super) struct TimerTaskFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct SecondRunningFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct FirstResumedFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct SecondResumedFlow {
    first: WorkerTask,
    second: WorkerTask,
}

impl TimerTaskFlow {
    pub(super) fn prepare(booted: &mut X86BootedKernel) -> Option<Self> {
        let (first, second) = setup::prepare(booted)?;
        Some(Self { first, second })
    }

    pub(super) fn expire_first_and_dispatch_second(
        self,
        booted: &mut X86BootedKernel,
        cpu: &PreemptedAgentCpu,
    ) -> Option<SecondRunningFlow> {
        transitions::expire_and_dispatch(booted, self.first, self.second, cpu, 0)?;
        Some(SecondRunningFlow {
            first: self.first,
            second: self.second,
        })
    }
}

impl SecondRunningFlow {
    pub(super) fn expire_second_and_dispatch_first(
        self,
        booted: &mut X86BootedKernel,
        cpu: &PreemptedAgentCpu,
    ) -> Option<FirstResumedFlow> {
        transitions::expire_and_dispatch(booted, self.second, self.first, cpu, 1)?;
        Some(FirstResumedFlow {
            first: self.first,
            second: self.second,
        })
    }
}

impl FirstResumedFlow {
    pub(super) fn yield_first_and_dispatch_second(
        self,
        booted: &mut X86BootedKernel,
        cpu: YieldedAgentCpu,
    ) -> Option<SecondResumedFlow> {
        transitions::yield_and_dispatch(booted, self.first, self.second, cpu, 1)?;
        Some(SecondResumedFlow {
            first: self.first,
            second: self.second,
        })
    }
}

impl SecondResumedFlow {
    pub(super) fn record_second_yield(
        self,
        booted: &mut X86BootedKernel,
        cpu: YieldedAgentCpu,
    ) -> bool {
        transitions::record_final_yield(booted, self.second, self.first, cpu)
    }
}
