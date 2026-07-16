//! Register decoding helpers for bounded native mailbox Agent Calls.
//!
//! This ABI-layer child maps canonical register payloads into core message
//! types. It performs no mailbox mutation and rejects every unsupported word.

use agent_kernel_core::{AgentId, MessageId, MessageKind, MessagePayload, TaskId};

use super::{
    decode_context_payload, ensure_extended_reserved_zero, ensure_reserved_zero,
    AgentCallDecodeError, AgentCallRequest, AGENT_CALL_MESSAGE_FAULT, AGENT_CALL_MESSAGE_NOTIFY,
    AGENT_CALL_MESSAGE_REQUEST, AGENT_CALL_MESSAGE_RESPONSE,
};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_send(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    ensure_send_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let kind = decode_message_kind(frame.r11)?;
    let payload = MessagePayload {
        task: (frame.r12 != 0).then(|| TaskId::new(frame.r12)),
        ..MessagePayload::empty()
    };
    Ok(AgentCallRequest::SendMessage {
        agent,
        task,
        image,
        nonce,
        recipient: AgentId::new(frame.r10),
        kind,
        payload,
    })
}

pub(super) fn decode_receive(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    ensure_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    Ok(AgentCallRequest::ReceiveMessage {
        agent,
        task,
        image,
        nonce,
    })
}

pub(super) fn decode_acknowledgement(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    ensure_acknowledgement_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::AcknowledgeMessage {
        agent,
        task,
        image,
        nonce,
        message: MessageId::new(frame.r10),
    })
}

fn ensure_send_reserved_zero(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(), AgentCallDecodeError> {
    if frame.r13 == 0 && frame.r14 == 0 && frame.r15 == 0 && frame.rbp == 0 {
        Ok(())
    } else {
        Err(AgentCallDecodeError::ReservedNotZero)
    }
}

fn ensure_acknowledgement_reserved_zero(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(), AgentCallDecodeError> {
    if frame.r11 == 0 {
        ensure_extended_reserved_zero(frame)
    } else {
        Err(AgentCallDecodeError::ReservedNotZero)
    }
}

fn decode_message_kind(value: u64) -> Result<MessageKind, AgentCallDecodeError> {
    match value {
        AGENT_CALL_MESSAGE_NOTIFY => Ok(MessageKind::Notify),
        AGENT_CALL_MESSAGE_REQUEST => Ok(MessageKind::Request),
        AGENT_CALL_MESSAGE_RESPONSE => Ok(MessageKind::Response),
        AGENT_CALL_MESSAGE_FAULT => Ok(MessageKind::Fault),
        _ => Err(AgentCallDecodeError::InvalidPayload),
    }
}

pub(super) const fn encode_message_kind(kind: MessageKind) -> u64 {
    match kind {
        MessageKind::Notify => AGENT_CALL_MESSAGE_NOTIFY,
        MessageKind::Request => AGENT_CALL_MESSAGE_REQUEST,
        MessageKind::Response => AGENT_CALL_MESSAGE_RESPONSE,
        MessageKind::Fault => AGENT_CALL_MESSAGE_FAULT,
    }
}
