//! Role-independent owned session for decoded native Agent calls.
//!
//! This CPU-layer module runs one validated ring-3 frame for a fresh PIT
//! quantum, then captures either the next call or a complete preemption frame.
//! Session nonce and transcript survive the timer boundary; semantic mutation
//! and reply choice remain outside the session.

mod replies;
mod runtime_memory;

use agent_kernel_x86_64::{
    agent_call::{AgentCallContext, AgentCallOperation, AgentCallRequest, AgentCallTranscript},
    context::SavedAgentFrame,
    native_runtime::NativeRunBoundary,
    runtime_reclamation::RuntimeReclamationLog,
    runtime_region::RuntimeRegionObservationLog,
};

use super::{call, runtime::AgentCpuRuntime, storage, FaultedAgentCpu, PreemptedAgentCpu};
use crate::agent_memory::PreparedAgentMemory;

pub(super) const MAX_AGENT_CALLS: usize = 44;

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
    pub(super) memory: PreparedAgentMemory,
    context: AgentCallContext,
    nonce: u64,
    transcript: AgentCallTranscript<MAX_AGENT_CALLS>,
    physical_quantum_generation: u8,
    restart_generation: u8,
    lazy_data_byte: u8,
    runtime_page_generation: u64,
    runtime_page_released: bool,
    runtime_page_observation: Option<u64>,
    runtime_region_generation: u64,
    runtime_regions_released: bool,
    runtime_region_observations: RuntimeRegionObservationLog,
    reclamation: RuntimeReclamationLog,
}

pub(crate) enum AgentRunOutcome {
    Call(PendingAgentCallCpu),
    Preempted(PreemptedAgentCpu),
    Fault(FaultedAgentCpu),
}

impl AgentRunOutcome {
    pub(crate) fn rebind_runtime(self, runtime: AgentCpuRuntime) -> Option<Self> {
        match self {
            Self::Call(cpu) => Some(Self::Call(cpu.rebind_runtime(runtime)?)),
            Self::Preempted(cpu) => Some(Self::Preempted(cpu.rebind_runtime(runtime)?)),
            Self::Fault(cpu) => Some(Self::Fault(cpu.rebind_runtime(runtime)?)),
        }
    }
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

    pub(super) const fn nonce(&self) -> Option<u64> {
        self.nonce
    }

    pub(super) const fn call_count(&self) -> usize {
        self.transcript.call_count()
    }

    pub(super) fn operations(&self) -> &[AgentCallOperation] {
        self.transcript.operations()
    }

    pub(super) fn return_offsets(&self) -> &[u32] {
        self.transcript.return_offsets()
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
        storage::begin_dispatch(self.runtime.transition, roots)?;
        self.runtime.arm_quantum_timer()?;
        let resumed = call::resume_owned(self.runtime.transition, &mut self.frame, roots, layout);
        let boundary = self.runtime.transition.run_boundary();
        self.runtime.finish_quantum_timer(boundary);
        resumed?;

        match boundary? {
            NativeRunBoundary::AgentCall => {
                let captured = call::capture(
                    self.runtime.transition,
                    self.runtime.kernel_stack,
                    roots,
                    layout,
                )?;
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

    fn rebind_runtime(mut self, runtime: AgentCpuRuntime) -> Option<Self> {
        if !runtime.accepts_memory(&self.session.memory) {
            return None;
        }
        self.session.runtime = runtime;
        Some(self)
    }

    pub(crate) fn authenticated_request(&self) -> Option<AgentCallRequest> {
        let nonce = self.session.progress.nonce?;
        self.session
            .context
            .authenticates(self.request, nonce)
            .then_some(self.request)
    }

    pub(crate) fn authenticated_namespace_path_buffer(
        &self,
    ) -> Option<agent_kernel_x86_64::namespace_path_buffer::NamespacePathBuffer> {
        let (root, generation) = match self.authenticated_request()? {
            AgentCallRequest::ResolveNamespacePathFromMemory {
                root, generation, ..
            } => (root, generation),
            _ => return None,
        };
        self.session
            .memory
            .snapshot_namespace_path(root, generation)
    }

    pub(crate) fn authenticated_typed_call_data_message(
        &self,
    ) -> Option<agent_kernel_x86_64::typed_call_data::CallDataMessage> {
        let generation = match self.authenticated_request()? {
            AgentCallRequest::CompareAndRebindNamespacePathFromMemory { generation, .. } => {
                (
                    generation,
                    agent_kernel_x86_64::typed_call_data::CallDataMessageKind::CompareAndRebindNamespacePath,
                )
            }
            AgentCallRequest::RotateAgentImageSignerFromMemory { generation, .. } => (
                generation,
                agent_kernel_x86_64::typed_call_data::CallDataMessageKind::RotateAgentImageSigner,
            ),
            _ => return None,
        };
        self.session
            .memory
            .snapshot_typed_call_data(generation.1, generation.0)
    }

    pub(crate) fn stage_durable_archive_preparation(
        &mut self,
        preparation: agent_kernel_x86_64::ata::NativeDurableArchivePreparation,
    ) -> bool {
        let valid = matches!(
            self.authenticated_request(),
            Some(AgentCallRequest::PrepareDurableArchive {
                archive_authority,
                storage_authority,
                through_sequence,
                generation,
                ..
            }) if preparation.caller().agent() == self.session.context.agent()
                && preparation.caller().task() == self.session.context.task()
                && preparation.caller().image() == self.session.context.image()
                && preparation.preflight().archive_authority() == archive_authority
                && preparation.preflight().storage_authority() == storage_authority
                && preparation.preflight().proposal().through_sequence() == through_sequence
                && preparation.call_data_generation() == generation
        );
        if !valid {
            return false;
        }
        self.session
            .memory
            .stage_durable_archive_request(&preparation.request_bytes())
    }

    pub(crate) fn authenticated_durable_archive_request(
        &self,
    ) -> Option<[u8; agent_kernel_x86_64::durable_archive_request::DURABLE_ARCHIVE_REQUEST_BYTES]>
    {
        let generation = match self.authenticated_request()? {
            AgentCallRequest::CommitDurableArchiveFromMemory { generation, .. } => generation,
            _ => return None,
        };
        self.session
            .memory
            .snapshot_durable_archive_request(generation)
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

    pub(crate) const fn runtime_region_generation(&self) -> u64 {
        self.runtime_region_generation
    }

    pub(crate) const fn runtime_regions_released(&self) -> bool {
        self.runtime_regions_released
    }

    pub(crate) const fn runtime_region_observations(&self) -> RuntimeRegionObservationLog {
        self.runtime_region_observations
    }

    pub(crate) const fn reclamation_log(&self) -> RuntimeReclamationLog {
        self.reclamation
    }
}
