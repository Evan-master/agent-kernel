//! Native ring-3 execution loop for one Core-selected Driver Invocation.
//!
//! This bare-metal adapter owns CPU dispatch and authenticated Driver Call
//! handling. Core owns lifecycle and Capability decisions; a supplied HAL
//! backend owns the only physical side effect.

mod calls;
mod state;

use agent_kernel_core::{
    AgentExecutionState, AgentId, CapabilityId, DriverCommandId, DriverCommandResult,
    DriverInvocationId, DriverInvocationStatus, EventKind, FaultKind,
};
use agent_kernel_hal::DriverBackend;
use agent_kernel_x86_64::{agent_call::AgentCallOperation, native_runtime::NativeAgentFault};

use crate::{
    agent_cpu::{CompletedAgentCpu, FaultedAgentCpu, PreemptedAgentCpu},
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
    fault: Option<NativeDriverFaultEvidence>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct DriverRecoveryAuthority {
    actor: AgentId,
    capability: CapabilityId,
}

impl DriverRecoveryAuthority {
    pub(crate) const fn new(actor: AgentId, capability: CapabilityId) -> Option<Self> {
        if actor.raw() == 0 || capability.raw() == 0 {
            None
        } else {
            Some(Self { actor, capability })
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeDriverFaultEvidence {
    detail: u64,
    offset: u32,
    nonce: u64,
    physical_quantum_generation: u8,
}

// One bounded execution loop owns either state directly; heap indirection would
// weaken the allocator-free native Driver contract.
#[allow(clippy::large_enum_variant)]
pub(super) enum DriverRunProgress {
    Preempted(PreemptedAgentCpu),
    Faulted(FaultedAgentCpu),
    Completed(CompletedAgentCpu),
}

pub(crate) fn run<B: DriverBackend>(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    driver: AgentId,
    expected_invocation: DriverInvocationId,
    recovery_authority: DriverRecoveryAuthority,
    backend: &mut B,
) -> Option<NativeDriverExecution> {
    let mut command = None;
    let mut dispatches = 0_u8;
    let mut quantum_expiries = 0_u8;
    let mut fault = None;
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
            DriverRunProgress::Faulted(cpu) => {
                if command.is_some() || fault.is_some() {
                    return None;
                }
                fault = Some(recover_fault(
                    booted,
                    runtime,
                    driver,
                    expected_invocation,
                    recovery_authority,
                    cpu,
                )?);
            }
            DriverRunProgress::Completed(completed) => {
                let command = command?;
                return Some(NativeDriverExecution {
                    completed,
                    command: command.id,
                    result: command.result,
                    dispatches,
                    quantum_expiries,
                    fault,
                });
            }
        }
    }
}

fn recover_fault(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    driver: AgentId,
    invocation: DriverInvocationId,
    authority: DriverRecoveryAuthority,
    faulted: FaultedAgentCpu,
) -> Option<NativeDriverFaultEvidence> {
    let context = faulted.context();
    let fault = faulted.fault();
    let detail = fault.detail();
    let offset = faulted.fault_offset()?;
    let nonce = faulted.call_nonce()?;
    let physical_quantum_generation = faulted.physical_quantum_generation();
    if context.agent() != driver
        || context.driver_invocation() != Some(invocation)
        || fault != NativeAgentFault::InvalidOpcode
        || detail != u64::from(NativeAgentFault::InvalidOpcode.vector())
        || faulted.restart_generation() != 0
        || physical_quantum_generation != 1
        || !faulted.runtime_memory_is_clear()
        || faulted.call_count() != 2
        || faulted.operations()
            != [
                AgentCallOperation::DescribeContext,
                AgentCallOperation::InspectDriverInvocation,
            ]
    {
        return None;
    }

    let transition = booted
        .kernel_mut()
        .sys_fault_driver_invocation(driver, invocation, FaultKind::ExecutionTrap, detail)
        .ok()?;
    let record = booted
        .kernel()
        .driver_invocations()
        .iter()
        .find(|record| record.id == invocation)?;
    let execution = booted
        .kernel()
        .execution_contexts()
        .iter()
        .find(|record| record.agent == driver)?;
    if transition.kind != EventKind::DriverInvocationFaulted
        || transition.driver_invocation != Some(invocation)
        || transition.fault_kind != Some(FaultKind::ExecutionTrap)
        || transition.fault_detail != Some(detail)
        || record.status != DriverInvocationStatus::Faulted
        || record.restart_generation != 0
        || execution.state != AgentExecutionState::Faulted
        || execution.driver_invocation != Some(invocation)
    {
        return None;
    }

    let restarted = faulted.restart()?;
    if restarted.context() != context {
        return None;
    }
    let recovery = booted
        .kernel_mut()
        .sys_recover_driver_invocation(authority.actor, authority.capability, driver, invocation)
        .ok()?;
    let record = booted
        .kernel()
        .driver_invocations()
        .iter()
        .find(|record| record.id == invocation)?;
    let execution = booted
        .kernel()
        .execution_contexts()
        .iter()
        .find(|record| record.agent == driver)?;
    if recovery.kind != EventKind::DriverInvocationRecovered
        || recovery.agent != authority.actor
        || recovery.capability != Some(authority.capability)
        || recovery.target_agent != Some(driver)
        || recovery.driver_invocation != Some(invocation)
        || record.status != DriverInvocationStatus::Queued
        || record.restart_generation != 1
        || execution.state != AgentExecutionState::Idle
        || runtime.register_prepared(restarted).is_some()
    {
        return None;
    }

    Some(NativeDriverFaultEvidence {
        detail,
        offset,
        nonce,
        physical_quantum_generation,
    })
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

    pub(crate) const fn fault(&self) -> Option<NativeDriverFaultEvidence> {
        self.fault
    }

    pub(crate) const fn completed(&self) -> &CompletedAgentCpu {
        &self.completed
    }

    pub(crate) fn into_completed(self) -> CompletedAgentCpu {
        self.completed
    }
}

impl NativeDriverFaultEvidence {
    pub(crate) const fn detail(self) -> u64 {
        self.detail
    }

    pub(crate) const fn offset(self) -> u32 {
        self.offset
    }

    pub(crate) const fn nonce(self) -> u64 {
        self.nonce
    }

    pub(crate) const fn physical_quantum_generation(self) -> u8 {
        self.physical_quantum_generation
    }
}
