//! Native ring-3 execution loop for one Core-selected Driver Invocation.
//!
//! This bare-metal adapter owns CPU dispatch and authenticated Driver Call
//! handling. Core owns lifecycle and Capability decisions; a supplied HAL
//! backend owns the only physical side effect.

mod calls;
mod state;

use agent_kernel_core::{
    AgentId, DriverCommandId, DriverCommandResult, DriverInvocationId, DriverInvocationStatus,
    EventKind,
};
use agent_kernel_hal::DriverBackend;

use crate::{
    agent_cpu::{CompletedAgentCpu, PreemptedAgentCpu},
    native_agent_runtime::{NativeAgentContext, NativeAgentRuntime},
    X86BootedKernel,
};

const DRIVER_QUANTUM: u64 = 1;

#[derive(Copy, Clone)]
pub(super) struct CommandEvidence {
    id: DriverCommandId,
    result: DriverCommandResult,
}

pub(crate) struct NativeDriverExecution {
    completed: CompletedAgentCpu,
    command: DriverCommandId,
    result: DriverCommandResult,
    dispatches: u8,
    quantum_expiries: u8,
}

// One bounded execution loop owns either state directly; heap indirection would
// weaken the allocator-free native Driver contract.
#[allow(clippy::large_enum_variant)]
pub(super) enum DriverRunProgress {
    Preempted(PreemptedAgentCpu),
    Completed(CompletedAgentCpu),
}

pub(crate) fn run<B: DriverBackend>(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    driver: AgentId,
    expected_invocation: DriverInvocationId,
    backend: &mut B,
) -> Option<NativeDriverExecution> {
    let mut command = None;
    let mut dispatches = 0_u8;
    let mut quantum_expiries = 0_u8;
    loop {
        let dispatched = runtime.dispatch_next_driver(booted, driver, DRIVER_QUANTUM)?;
        if dispatched.invocation() != expected_invocation {
            return None;
        }
        dispatches = dispatches.checked_add(1)?;
        let outcome = match dispatched.into_context() {
            NativeAgentContext::Prepared(cpu) => cpu.run_until_boundary()?,
            NativeAgentContext::Preempted(cpu) => cpu.resume_until_boundary()?,
            NativeAgentContext::WaitingCall(_)
            | NativeAgentContext::YieldedCall(_)
            | NativeAgentContext::RecoveredFault(_) => return None,
        };
        match calls::continue_run(booted, backend, &mut command, outcome)? {
            DriverRunProgress::Preempted(cpu) => {
                expire_quantum(booted, runtime, driver, expected_invocation, cpu)?;
                quantum_expiries = quantum_expiries.checked_add(1)?;
            }
            DriverRunProgress::Completed(completed) => {
                let command = command?;
                return Some(NativeDriverExecution {
                    completed,
                    command: command.id,
                    result: command.result,
                    dispatches,
                    quantum_expiries,
                });
            }
        }
    }
}

fn expire_quantum(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    driver: AgentId,
    invocation: DriverInvocationId,
    cpu: PreemptedAgentCpu,
) -> Option<()> {
    let context = cpu.context();
    if context.agent() != driver
        || context.driver_invocation() != Some(invocation)
        || cpu.tick_count() != 1
    {
        return None;
    }
    let transition = booted
        .kernel_mut()
        .sys_tick_driver_invocation(driver, invocation)
        .ok()?;
    let record = booted
        .kernel()
        .driver_invocations()
        .iter()
        .find(|record| record.id == invocation)?;
    if transition.kind != EventKind::DriverInvocationQuantumExpired
        || transition.driver_invocation != Some(invocation)
        || record.status != DriverInvocationStatus::Queued
        || record.quantum_remaining != 0
        || runtime.park_preempted(cpu).is_some()
    {
        return None;
    }
    Some(())
}

impl NativeDriverExecution {
    pub(crate) const fn command(&self) -> DriverCommandId {
        self.command
    }

    pub(crate) const fn result(&self) -> DriverCommandResult {
        self.result
    }

    pub(crate) const fn dispatches(&self) -> u8 {
        self.dispatches
    }

    pub(crate) const fn quantum_expiries(&self) -> u8 {
        self.quantum_expiries
    }

    pub(crate) const fn completed(&self) -> &CompletedAgentCpu {
        &self.completed
    }

    pub(crate) fn into_completed(self) -> CompletedAgentCpu {
        self.completed
    }
}
