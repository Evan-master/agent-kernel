//! Shared owned-frame mechanics for native mailbox Agent Call sequences.
//!
//! This CPU-layer module establishes trusted DescribeContext state and resumes
//! one validated call at a time. Sender and receiver children own operation
//! ordering; all mailbox and task mutations remain in the boot adapter.

mod receiver;
mod sender;

use agent_kernel_x86_64::{
    agent_call::{AgentCallContext, AgentCallRequest},
    context::{PrivilegeInterruptStackFrame, SavedAgentFrame},
};

use super::{call, runtime::AgentCpuRuntime, storage, PreemptedAgentCpu};
use crate::agent_memory::PreparedAgentMemory;

pub(crate) use receiver::{
    AcknowledgedMessageAcknowledgementCpu, AcknowledgedMessageReceiveCpu,
    AcknowledgedReceiverResultCpu, CompletedMailboxReceiverCpu, RequestedMessageAcknowledgementCpu,
    RequestedMessageReceiveCpu, RequestedReceiverResultCpu, WaitingMessageReceiveCpu,
};
pub(crate) use sender::{
    AcknowledgedMessageSendCpu, AcknowledgedSenderResultCpu, CompletedMailboxSenderCpu,
    RequestedMessageSendCpu, RequestedSenderResultCpu, RequestedSenderYieldCpu,
    YieldedMailboxSenderCpu,
};

pub(super) struct MailboxCallSession {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    frame: SavedAgentFrame,
    context: AgentCallContext,
    nonce: u64,
    describe_return_offset: u32,
}

impl MailboxCallSession {
    pub(super) fn begin(mut cpu: PreemptedAgentCpu) -> Option<Self> {
        let roots = cpu.memory.roots();
        let layout = cpu.memory.layout();
        storage::begin_dispatch(roots)?;
        if !cpu.memory.release_for_agent_call() {
            return None;
        }
        call::resume_owned(&mut cpu.frame, roots, layout)?;
        let describe = call::capture(cpu.runtime.kernel_stack, roots, layout)?;
        let nonce = match describe.request() {
            AgentCallRequest::DescribeContext { nonce } => nonce,
            _ => return None,
        };
        let describe_return_offset = describe.return_offset();
        let mut frame = describe.into_frame();
        cpu.context
            .encode_describe_reply(frame.frame_mut(), nonce)
            .ok()?;
        Some(Self {
            memory: cpu.memory,
            runtime: cpu.runtime,
            frame,
            context: cpu.context,
            nonce,
            describe_return_offset,
        })
    }

    pub(super) fn resume_next(mut self) -> Option<(Self, AgentCallRequest, u32)> {
        let roots = self.memory.roots();
        let layout = self.memory.layout();
        storage::begin_dispatch(roots)?;
        call::resume_owned(&mut self.frame, roots, layout)?;
        let captured = call::capture(self.runtime.kernel_stack, roots, layout)?;
        let request = captured.request();
        let return_offset = captured.return_offset();
        self.frame = captured.into_frame();
        Some((self, request, return_offset))
    }

    pub(super) const fn context(&self) -> AgentCallContext {
        self.context
    }

    pub(super) const fn nonce(&self) -> u64 {
        self.nonce
    }

    pub(super) const fn describe_return_offset(&self) -> u32 {
        self.describe_return_offset
    }

    pub(super) fn frame_mut(&mut self) -> &mut PrivilegeInterruptStackFrame {
        self.frame.frame_mut()
    }

    pub(super) fn agent_call_is_released(&self) -> bool {
        self.memory.agent_call_is_released()
    }
}
