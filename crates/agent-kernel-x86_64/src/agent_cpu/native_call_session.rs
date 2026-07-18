//! Role-independent owned session for decoded native Agent calls.
//!
//! This CPU-layer module runs one validated ring-3 frame for a fresh PIT
//! quantum, then captures either the next call or a complete preemption frame.
//! Session nonce and transcript survive the timer boundary; semantic mutation
//! and reply choice remain outside the session.

mod replies;

use agent_kernel_core::{MemoryCellId, MemoryValue, ResourceId};
use agent_kernel_x86_64::{
    agent_call::{AgentCallContext, AgentCallRequest, AgentCallTranscript},
    context::SavedAgentFrame,
    native_runtime::NativeRunBoundary,
    runtime_page::{RuntimePageRelease, RuntimePageReservation},
};

use super::{call, runtime::AgentCpuRuntime, storage, FaultedAgentCpu, PreemptedAgentCpu};
use crate::{agent_memory::PreparedAgentMemory, pit_timer};

pub(super) const MAX_AGENT_CALLS: usize = 18;

struct AgentCallSession {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    frame: SavedAgentFrame,
    context: AgentCallContext,
    progress: AgentCallProgress,
}

pub(super) struct AgentCallProgress {
    nonce: Option<u64>,
    transcript: AgentCallTranscript<MAX_AGENT_CALLS>,
}

pub(crate) struct PendingAgentCallCpu {
    session: AgentCallSession,
    request: AgentCallRequest,
}

pub(crate) struct ResumableAgentCpu(AgentCallSession);

pub(crate) struct WaitingAgentCallCpu {
    pending: PendingAgentCallCpu,
    waiter: agent_kernel_core::WaiterId,
}

pub(crate) struct CompletedAgentCpu {
    context: AgentCallContext,
    nonce: u64,
    transcript: AgentCallTranscript<MAX_AGENT_CALLS>,
    physical_quantum_generation: u8,
    restart_generation: u8,
    lazy_data_byte: u8,
    runtime_page_generation: u64,
    runtime_page_released: bool,
    runtime_page_observation: Option<u64>,
}

pub(crate) enum AgentRunOutcome {
    Call(PendingAgentCallCpu),
    Preempted(PreemptedAgentCpu),
    Fault(FaultedAgentCpu),
}

impl PreemptedAgentCpu {
    pub(crate) fn resume_until_boundary(mut self) -> Option<AgentRunOutcome> {
        if !self.memory.agent_call_is_released() && !self.memory.release_for_agent_call() {
            return None;
        }
        AgentCallSession {
            memory: self.memory,
            runtime: self.runtime,
            frame: self.frame,
            context: self.context,
            progress: self.progress,
        }
        .resume_until_boundary()
    }
}

impl AgentCallProgress {
    pub(super) const fn new() -> Self {
        Self {
            nonce: None,
            transcript: AgentCallTranscript::new(),
        }
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.transcript.is_empty()
    }
}

impl AgentCallSession {
    fn with_request(
        mut self,
        request: AgentCallRequest,
        return_offset: u32,
    ) -> Option<PendingAgentCallCpu> {
        self.progress
            .transcript
            .record(request.operation(), return_offset)
            .ok()?;
        Some(PendingAgentCallCpu {
            session: self,
            request,
        })
    }

    fn resume_until_boundary(mut self) -> Option<AgentRunOutcome> {
        let roots = self.memory.roots();
        let layout = self.memory.layout();
        storage::begin_dispatch(roots)?;
        pit_timer::arm(super::assembly::agent_kernel_agent_timer_irq_stub)?;
        let resumed = call::resume_owned(&mut self.frame, roots, layout);
        pit_timer::disarm();
        resumed?;

        match storage::run_boundary()? {
            NativeRunBoundary::AgentCall => {
                let captured = call::capture(self.runtime.kernel_stack, roots, layout)?;
                let request = captured.request();
                let return_offset = captured.return_offset();
                self.frame = captured.into_frame();
                Some(AgentRunOutcome::Call(
                    self.with_request(request, return_offset)?,
                ))
            }
            NativeRunBoundary::QuantumExpired => {
                Some(AgentRunOutcome::Preempted(PreemptedAgentCpu::capture(
                    self.memory,
                    self.runtime,
                    self.context,
                    self.progress,
                    false,
                )?))
            }
            NativeRunBoundary::AgentFault(fault) => {
                Some(AgentRunOutcome::Fault(FaultedAgentCpu::capture(
                    self.memory,
                    self.runtime,
                    self.context,
                    self.progress,
                    fault,
                )?))
            }
        }
    }
}

impl PendingAgentCallCpu {
    pub(crate) const fn request(&self) -> AgentCallRequest {
        self.request
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.session.context
    }

    pub(crate) const fn call_count(&self) -> usize {
        self.session.progress.transcript.call_count()
    }

    pub(crate) fn authenticated_request(&self) -> Option<AgentCallRequest> {
        let nonce = self.session.progress.nonce?;
        self.session
            .context
            .authenticates(self.request, nonce)
            .then_some(self.request)
    }

    pub(crate) fn prepare_runtime_page_allocation(
        &mut self,
        resource: ResourceId,
    ) -> Option<(RuntimePageReservation, MemoryValue)> {
        self.session
            .memory
            .prepare_runtime_page_allocation(resource)
    }

    pub(crate) fn commit_runtime_page_allocation(
        &mut self,
        reservation: RuntimePageReservation,
        cell: MemoryCellId,
    ) -> bool {
        self.session
            .memory
            .commit_runtime_page_allocation(reservation, cell)
    }

    pub(crate) fn rollback_runtime_page_allocation(
        &mut self,
        reservation: RuntimePageReservation,
    ) -> bool {
        self.session
            .memory
            .rollback_runtime_page_allocation(reservation)
    }

    pub(crate) fn inspect_runtime_page(
        &mut self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
    ) -> Option<(u64, u64)> {
        self.session
            .memory
            .inspect_runtime_page(resource, cell, descriptor)
    }

    pub(crate) fn prepare_runtime_page_release(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
    ) -> Option<RuntimePageRelease> {
        self.session
            .memory
            .prepare_runtime_page_release(resource, cell, descriptor)
    }

    pub(crate) fn release_runtime_page(&mut self, release: RuntimePageRelease) -> bool {
        self.session.memory.release_runtime_page(release)
    }
}

impl ResumableAgentCpu {
    pub(super) fn from_repaired_fault(
        memory: PreparedAgentMemory,
        runtime: AgentCpuRuntime,
        frame: SavedAgentFrame,
        context: AgentCallContext,
        progress: AgentCallProgress,
    ) -> Self {
        Self(AgentCallSession {
            memory,
            runtime,
            frame,
            context,
            progress,
        })
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.0.context
    }

    pub(crate) fn resume_until_boundary(self) -> Option<AgentRunOutcome> {
        self.0.resume_until_boundary()
    }
}

impl WaitingAgentCallCpu {
    pub(crate) const fn context(&self) -> AgentCallContext {
        self.pending.session.context
    }

    pub(crate) const fn waiter(&self) -> agent_kernel_core::WaiterId {
        self.waiter
    }
}

impl CompletedAgentCpu {
    pub(crate) const fn context(&self) -> AgentCallContext {
        self.context
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.nonce
    }

    pub(crate) const fn call_count(&self) -> usize {
        self.transcript.call_count()
    }

    pub(crate) const fn address_space_switch_count(&self) -> usize {
        self.transcript.address_space_switch_count()
    }

    pub(crate) fn operations(&self) -> &[agent_kernel_x86_64::agent_call::AgentCallOperation] {
        self.transcript.operations()
    }

    pub(crate) fn return_offsets(&self) -> &[u32] {
        self.transcript.return_offsets()
    }

    pub(crate) const fn physical_quantum_generation(&self) -> u8 {
        self.physical_quantum_generation
    }

    pub(crate) const fn restart_generation(&self) -> u8 {
        self.restart_generation
    }

    pub(crate) const fn lazy_data_byte(&self) -> u8 {
        self.lazy_data_byte
    }

    pub(crate) const fn runtime_page_generation(&self) -> u64 {
        self.runtime_page_generation
    }

    pub(crate) const fn runtime_page_released(&self) -> bool {
        self.runtime_page_released
    }

    pub(crate) const fn runtime_page_observation(&self) -> Option<u64> {
        self.runtime_page_observation
    }
}
