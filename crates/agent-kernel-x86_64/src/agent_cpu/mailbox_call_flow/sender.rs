//! Four-call CPU type states for the native mailbox sender Worker.
//!
//! This CPU-layer child preserves the owned frame across result, send, and
//! completion calls. Semantic task and mailbox mutation stays outside it.

use agent_kernel_core::{AgentId, MessageId, MessageKind, MessagePayload, TaskResult};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use super::MailboxCallSession;
use crate::agent_cpu::PreemptedAgentCpu;

pub(crate) struct RequestedSenderResultCpu {
    session: MailboxCallSession,
    result: TaskResult,
    result_return_offset: u32,
}

pub(crate) struct AcknowledgedSenderResultCpu(RequestedSenderResultCpu);

pub(crate) struct RequestedMessageSendCpu {
    result: RequestedSenderResultCpu,
    recipient: AgentId,
    kind: MessageKind,
    payload: MessagePayload,
    send_return_offset: u32,
}

pub(crate) struct AcknowledgedMessageSendCpu {
    request: RequestedMessageSendCpu,
    message: MessageId,
}

pub(crate) struct CompletedMailboxSenderCpu {
    context: AgentCallContext,
    nonce: u64,
    result: TaskResult,
    recipient: AgentId,
    message: MessageId,
    return_offsets: [u32; 4],
}

impl PreemptedAgentCpu {
    pub(crate) fn resume_until_sender_result(self) -> Option<RequestedSenderResultCpu> {
        let session = MailboxCallSession::begin(self)?;
        let (session, request, result_return_offset) = session.resume_next()?;
        let result = session
            .context()
            .match_task_result(request, session.nonce())?;
        Some(RequestedSenderResultCpu {
            session,
            result,
            result_return_offset,
        })
    }
}

impl RequestedSenderResultCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        2
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        4
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.session.context()
    }

    pub(crate) const fn result(&self) -> TaskResult {
        self.result
    }

    pub(crate) const fn describe_return_offset(&self) -> u32 {
        self.session.describe_return_offset()
    }

    pub(crate) const fn result_return_offset(&self) -> u32 {
        self.result_return_offset
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.session.nonce()
    }

    pub(crate) fn acknowledge(mut self) -> Option<AcknowledgedSenderResultCpu> {
        let context = self.session.context();
        let nonce = self.session.nonce();
        context
            .encode_task_result_reply(self.session.frame_mut(), nonce)
            .ok()?;
        Some(AcknowledgedSenderResultCpu(self))
    }
}

impl AcknowledgedSenderResultCpu {
    pub(crate) fn resume_until_message_send(
        self,
        expected_recipient: AgentId,
    ) -> Option<RequestedMessageSendCpu> {
        let (session, request, send_return_offset) = self.0.session.resume_next()?;
        let (kind, payload) =
            session
                .context()
                .match_message_send(request, session.nonce(), expected_recipient)?;
        Some(RequestedMessageSendCpu {
            result: RequestedSenderResultCpu {
                session,
                result: self.0.result,
                result_return_offset: self.0.result_return_offset,
            },
            recipient: expected_recipient,
            kind,
            payload,
            send_return_offset,
        })
    }
}

impl RequestedMessageSendCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        3
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        6
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.result.context()
    }

    pub(crate) const fn recipient(&self) -> AgentId {
        self.recipient
    }

    pub(crate) const fn kind(&self) -> MessageKind {
        self.kind
    }

    pub(crate) const fn payload(&self) -> MessagePayload {
        self.payload
    }

    pub(crate) const fn send_return_offset(&self) -> u32 {
        self.send_return_offset
    }

    pub(crate) fn acknowledge(mut self, message: MessageId) -> Option<AcknowledgedMessageSendCpu> {
        let context = self.result.session.context();
        let nonce = self.result.session.nonce();
        context
            .encode_message_send_reply(self.result.session.frame_mut(), nonce, message)
            .ok()?;
        Some(AcknowledgedMessageSendCpu {
            request: self,
            message,
        })
    }
}

impl AcknowledgedMessageSendCpu {
    pub(crate) fn resume_until_completion(self) -> Option<CompletedMailboxSenderCpu> {
        let (session, request, completion_return_offset) =
            self.request.result.session.resume_next()?;
        if !session
            .context()
            .matches_completion(request, session.nonce())
        {
            return None;
        }
        Some(CompletedMailboxSenderCpu {
            context: session.context(),
            nonce: session.nonce(),
            result: self.request.result.result,
            recipient: self.request.recipient,
            message: self.message,
            return_offsets: [
                session.describe_return_offset(),
                self.request.result.result_return_offset,
                self.request.send_return_offset,
                completion_return_offset,
            ],
        })
    }
}

impl CompletedMailboxSenderCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        4
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        8
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

    pub(crate) const fn return_offsets(&self) -> [u32; 4] {
        self.return_offsets
    }
}
