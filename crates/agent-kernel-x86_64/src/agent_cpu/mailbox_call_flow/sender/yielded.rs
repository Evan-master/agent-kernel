//! Owned CPU states for one native sender Yield and later completion.
//!
//! This CPU-layer child captures the canonical Yield request, writes its reply
//! only after semantic acknowledgement, and preserves the non-Copy call frame
//! while another Agent runs. Scheduler mutation remains outside this module.

use agent_kernel_core::{AgentId, MessageId, TaskResult};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use super::{AcknowledgedMessageSendCpu, RequestedMessageSendCpu, RequestedSenderResultCpu};

pub(crate) struct RequestedSenderYieldCpu {
    request: RequestedMessageSendCpu,
    message: MessageId,
    yield_return_offset: u32,
}

pub(crate) struct YieldedMailboxSenderCpu(RequestedSenderYieldCpu);

pub(crate) struct CompletedMailboxSenderCpu {
    context: AgentCallContext,
    nonce: u64,
    result: TaskResult,
    recipient: AgentId,
    message: MessageId,
    return_offsets: [u32; 5],
}

impl AcknowledgedMessageSendCpu {
    pub(crate) fn resume_until_yield(self) -> Option<RequestedSenderYieldCpu> {
        let (session, request, yield_return_offset) = self.request.result.session.resume_next()?;
        if !session.context().matches_yield(request, session.nonce()) {
            return None;
        }
        Some(RequestedSenderYieldCpu {
            request: RequestedMessageSendCpu {
                result: RequestedSenderResultCpu {
                    session,
                    result: self.request.result.result,
                    result_return_offset: self.request.result.result_return_offset,
                },
                recipient: self.request.recipient,
                kind: self.request.kind,
                payload: self.request.payload,
                send_return_offset: self.request.send_return_offset,
            },
            message: self.message,
            yield_return_offset,
        })
    }
}

impl RequestedSenderYieldCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        4
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        8
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.request.result.context()
    }

    pub(crate) const fn message(&self) -> MessageId {
        self.message
    }

    pub(crate) const fn yield_return_offset(&self) -> u32 {
        self.yield_return_offset
    }

    pub(crate) fn acknowledge(mut self) -> Option<YieldedMailboxSenderCpu> {
        let context = self.request.result.session.context();
        let nonce = self.request.result.session.nonce();
        context
            .encode_yield_reply(self.request.result.session.frame_mut(), nonce)
            .ok()?;
        Some(YieldedMailboxSenderCpu(self))
    }
}

impl YieldedMailboxSenderCpu {
    pub(crate) const fn context(&self) -> AgentCallContext {
        self.0.context()
    }

    pub(crate) fn resume_until_completion(self) -> Option<CompletedMailboxSenderCpu> {
        let (session, request, completion_return_offset) =
            self.0.request.result.session.resume_next()?;
        if !session
            .context()
            .matches_completion(request, session.nonce())
        {
            return None;
        }
        Some(CompletedMailboxSenderCpu {
            context: session.context(),
            nonce: session.nonce(),
            result: self.0.request.result.result,
            recipient: self.0.request.recipient,
            message: self.0.message,
            return_offsets: [
                session.describe_return_offset(),
                self.0.request.result.result_return_offset,
                self.0.request.send_return_offset,
                self.0.yield_return_offset,
                completion_return_offset,
            ],
        })
    }
}

impl CompletedMailboxSenderCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        5
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        10
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.context
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.nonce
    }

    pub(crate) const fn result(&self) -> TaskResult {
        self.result
    }

    pub(crate) const fn recipient(&self) -> AgentId {
        self.recipient
    }

    pub(crate) const fn message(&self) -> MessageId {
        self.message
    }

    pub(crate) const fn return_offsets(&self) -> [u32; 5] {
        self.return_offsets
    }
}
