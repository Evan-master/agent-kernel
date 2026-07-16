//! Five-call CPU type states for the native mailbox receiver Worker.
//!
//! This CPU-layer child preserves the owned frame across receive,
//! acknowledgement, result, and completion calls without mutating core state.

mod completed;

use agent_kernel_core::{MessageId, MessageRecord, TaskResult};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use super::MailboxCallSession;
use crate::agent_cpu::PreemptedAgentCpu;

pub(crate) use completed::CompletedMailboxReceiverCpu;

pub(crate) struct RequestedMessageReceiveCpu {
    session: MailboxCallSession,
    receive_return_offset: u32,
}

pub(crate) struct AcknowledgedMessageReceiveCpu {
    request: RequestedMessageReceiveCpu,
    message: MessageRecord,
}

pub(crate) struct RequestedMessageAcknowledgementCpu {
    received: AcknowledgedMessageReceiveCpu,
    acknowledgement_return_offset: u32,
}

pub(crate) struct AcknowledgedMessageAcknowledgementCpu(RequestedMessageAcknowledgementCpu);

pub(crate) struct RequestedReceiverResultCpu {
    acknowledged: RequestedMessageAcknowledgementCpu,
    result: TaskResult,
    result_return_offset: u32,
}

pub(crate) struct AcknowledgedReceiverResultCpu(RequestedReceiverResultCpu);

impl PreemptedAgentCpu {
    pub(crate) fn resume_until_message_receive(self) -> Option<RequestedMessageReceiveCpu> {
        let session = MailboxCallSession::begin(self)?;
        let (session, request, receive_return_offset) = session.resume_next()?;
        if !session
            .context()
            .matches_message_receive(request, session.nonce())
        {
            return None;
        }
        Some(RequestedMessageReceiveCpu {
            session,
            receive_return_offset,
        })
    }
}

impl RequestedMessageReceiveCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        2
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        4
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.session.context()
    }

    pub(crate) const fn receive_return_offset(&self) -> u32 {
        self.receive_return_offset
    }

    pub(crate) fn acknowledge(
        mut self,
        message: MessageRecord,
    ) -> Option<AcknowledgedMessageReceiveCpu> {
        let context = self.session.context();
        let nonce = self.session.nonce();
        context
            .encode_message_receive_reply(self.session.frame_mut(), nonce, message)
            .ok()?;
        Some(AcknowledgedMessageReceiveCpu {
            request: self,
            message,
        })
    }
}

impl AcknowledgedMessageReceiveCpu {
    pub(crate) fn resume_until_message_acknowledgement(
        self,
    ) -> Option<RequestedMessageAcknowledgementCpu> {
        let (session, request, acknowledgement_return_offset) =
            self.request.session.resume_next()?;
        if !session.context().matches_message_acknowledgement(
            request,
            session.nonce(),
            self.message.id,
        ) {
            return None;
        }
        Some(RequestedMessageAcknowledgementCpu {
            received: AcknowledgedMessageReceiveCpu {
                request: RequestedMessageReceiveCpu {
                    session,
                    receive_return_offset: self.request.receive_return_offset,
                },
                message: self.message,
            },
            acknowledgement_return_offset,
        })
    }
}

impl RequestedMessageAcknowledgementCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        3
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        6
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.received.request.context()
    }

    pub(crate) const fn message(&self) -> MessageId {
        self.received.message.id
    }

    pub(crate) fn acknowledge(mut self) -> Option<AcknowledgedMessageAcknowledgementCpu> {
        let context = self.received.request.session.context();
        let nonce = self.received.request.session.nonce();
        context
            .encode_message_acknowledgement_reply(self.received.request.session.frame_mut(), nonce)
            .ok()?;
        Some(AcknowledgedMessageAcknowledgementCpu(self))
    }
}

impl AcknowledgedMessageAcknowledgementCpu {
    pub(crate) fn resume_until_receiver_result(self) -> Option<RequestedReceiverResultCpu> {
        let (session, request, result_return_offset) =
            self.0.received.request.session.resume_next()?;
        let result = session
            .context()
            .match_task_result(request, session.nonce())?;
        Some(RequestedReceiverResultCpu {
            acknowledged: RequestedMessageAcknowledgementCpu {
                received: AcknowledgedMessageReceiveCpu {
                    request: RequestedMessageReceiveCpu {
                        session,
                        receive_return_offset: self.0.received.request.receive_return_offset,
                    },
                    message: self.0.received.message,
                },
                acknowledgement_return_offset: self.0.acknowledgement_return_offset,
            },
            result,
            result_return_offset,
        })
    }
}

impl RequestedReceiverResultCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        4
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        8
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.acknowledged.context()
    }

    pub(crate) const fn result(&self) -> TaskResult {
        self.result
    }

    pub(crate) fn acknowledge(mut self) -> Option<AcknowledgedReceiverResultCpu> {
        let context = self.acknowledged.received.request.session.context();
        let nonce = self.acknowledged.received.request.session.nonce();
        context
            .encode_task_result_reply(
                self.acknowledged.received.request.session.frame_mut(),
                nonce,
            )
            .ok()?;
        Some(AcknowledgedReceiverResultCpu(self))
    }
}
