//! Role-independent owned session for decoded native Agent calls.
//!
//! This CPU-layer module resumes one validated ring-3 frame at a time, captures
//! the next call, and appends physical transcript evidence. Semantic mutation
//! and reply choice remain outside the session.

mod replies;

use agent_kernel_x86_64::{
    agent_call::{AgentCallContext, AgentCallRequest, AgentCallTranscript},
    context::SavedAgentFrame,
};

use super::{call, runtime::AgentCpuRuntime, storage, PreemptedAgentCpu};
use crate::agent_memory::PreparedAgentMemory;

pub(super) const MAX_AGENT_CALLS: usize = 8;

struct AgentCallSession {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    frame: SavedAgentFrame,
    context: AgentCallContext,
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
}

impl PreemptedAgentCpu {
    pub(crate) fn resume_until_agent_call(mut self) -> Option<PendingAgentCallCpu> {
        let roots = self.memory.roots();
        let layout = self.memory.layout();
        storage::begin_dispatch(roots)?;
        if !self.memory.release_for_agent_call() {
            return None;
        }
        call::resume_owned(&mut self.frame, roots, layout)?;
        let captured = call::capture(self.runtime.kernel_stack, roots, layout)?;
        let request = captured.request();
        let return_offset = captured.return_offset();
        AgentCallSession {
            memory: self.memory,
            runtime: self.runtime,
            frame: captured.into_frame(),
            context: self.context,
            nonce: None,
            transcript: AgentCallTranscript::new(),
        }
        .with_request(request, return_offset)
    }
}

impl AgentCallSession {
    fn with_request(
        mut self,
        request: AgentCallRequest,
        return_offset: u32,
    ) -> Option<PendingAgentCallCpu> {
        self.transcript
            .record(request.operation(), return_offset)
            .ok()?;
        Some(PendingAgentCallCpu {
            session: self,
            request,
        })
    }

    fn resume_next(mut self) -> Option<PendingAgentCallCpu> {
        let roots = self.memory.roots();
        let layout = self.memory.layout();
        storage::begin_dispatch(roots)?;
        call::resume_owned(&mut self.frame, roots, layout)?;
        let captured = call::capture(self.runtime.kernel_stack, roots, layout)?;
        let request = captured.request();
        let return_offset = captured.return_offset();
        self.frame = captured.into_frame();
        self.with_request(request, return_offset)
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
        self.session.transcript.call_count()
    }

    pub(crate) fn authenticated_request(&self) -> Option<AgentCallRequest> {
        let nonce = self.session.nonce?;
        self.session
            .context
            .authenticates(self.request, nonce)
            .then_some(self.request)
    }
}

impl ResumableAgentCpu {
    pub(crate) const fn context(&self) -> AgentCallContext {
        self.0.context
    }

    pub(crate) fn resume_until_agent_call(self) -> Option<PendingAgentCallCpu> {
        self.0.resume_next()
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
}
