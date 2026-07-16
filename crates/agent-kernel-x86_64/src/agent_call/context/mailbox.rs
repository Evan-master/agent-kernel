//! Trusted matching and canonical replies for native mailbox Agent Calls.
//!
//! This ABI-layer child binds requests to scheduler-owned call identity and
//! emits only the fixed message fields supported by physical V0 Agents.

use agent_kernel_core::{
    AgentId, MessageId, MessageKind, MessagePayload, MessageRecord, MessageStatus, TaskId,
};

use super::AgentCallContext;
use crate::{
    agent_call::{
        mailbox::encode_message_kind, AgentCallDecodeError, AgentCallRequest,
        AGENT_CALL_ACKNOWLEDGE_MESSAGE, AGENT_CALL_RECEIVE_MESSAGE, AGENT_CALL_SEND_MESSAGE,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_message_send_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        message: MessageId,
    ) -> Result<(), AgentCallDecodeError> {
        if message.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_SEND_MESSAGE)?;
        frame.r10 = message.raw();
        Ok(())
    }

    pub fn encode_message_receive_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        message: MessageRecord,
    ) -> Result<(), AgentCallDecodeError> {
        if message.id.raw() == 0
            || message.sender.raw() == 0
            || message.recipient != self.agent
            || message.status != MessageStatus::Received
            || !native_message_payload_supported(message.payload)
        {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RECEIVE_MESSAGE)?;
        frame.r10 = message.id.raw();
        frame.r11 = message.sender.raw();
        frame.r12 = encode_message_kind(message.kind);
        frame.r13 = message.payload.task.map_or(0, TaskId::raw);
        Ok(())
    }

    pub fn encode_message_acknowledgement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_reply(frame, nonce, AGENT_CALL_ACKNOWLEDGE_MESSAGE)
    }

    pub fn match_message_send(
        self,
        request: AgentCallRequest,
        expected_nonce: u64,
        expected_recipient: AgentId,
    ) -> Option<(MessageKind, MessagePayload)> {
        match request {
            AgentCallRequest::SendMessage {
                agent,
                task,
                image,
                nonce,
                recipient,
                kind,
                payload,
            } if recipient == expected_recipient
                && self.matches_identity(agent, task, image, nonce, expected_nonce) =>
            {
                Some((kind, payload))
            }
            _ => None,
        }
    }

    pub fn matches_message_receive(self, request: AgentCallRequest, expected_nonce: u64) -> bool {
        matches!(
            request,
            AgentCallRequest::ReceiveMessage {
                agent,
                task,
                image,
                nonce,
            } if self.matches_identity(agent, task, image, nonce, expected_nonce)
        )
    }

    pub fn matches_message_acknowledgement(
        self,
        request: AgentCallRequest,
        expected_nonce: u64,
        expected_message: MessageId,
    ) -> bool {
        matches!(
            request,
            AgentCallRequest::AcknowledgeMessage {
                agent,
                task,
                image,
                nonce,
                message,
            } if message == expected_message
                && self.matches_identity(agent, task, image, nonce, expected_nonce)
        )
    }
}

const fn native_message_payload_supported(payload: MessagePayload) -> bool {
    (payload.task.is_none() || matches!(payload.task, Some(task) if task.raw() != 0))
        && payload.resource.is_none()
        && payload.capability.is_none()
        && payload.intent.is_none()
        && payload.action.is_none()
        && payload.fault.is_none()
}
