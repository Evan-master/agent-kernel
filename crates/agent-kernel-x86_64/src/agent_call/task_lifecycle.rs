//! Strict register decoding for Intent and Task lifecycle Agent Calls.
//!
//! This x86 ABI module converts bounded wire values into AgentOS-native ids
//! and enums. Trusted kernel layers retain every authority and state decision.

use agent_kernel_core::{
    AgentId, CapabilityId, IntentId, IntentKind, ResourceId, TaskId, VerificationRequirement,
};

use super::{
    decode_context_payload, AgentCallDecodeError, AgentCallRequest, AGENT_CALL_INTENT_ACT,
    AGENT_CALL_INTENT_CHECKPOINT, AGENT_CALL_INTENT_OBSERVE, AGENT_CALL_INTENT_ROLLBACK,
    AGENT_CALL_INTENT_VERIFY, AGENT_CALL_VERIFICATION_OPTIONAL, AGENT_CALL_VERIFICATION_REQUIRED,
};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_declare(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let kind = decode_kind(frame.r12).ok_or(AgentCallDecodeError::InvalidPayload)?;
    let verification =
        decode_verification(frame.r13).ok_or(AgentCallDecodeError::InvalidPayload)?;
    Ok(AgentCallRequest::DeclareIntent {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        resource: ResourceId::new(frame.r11),
        kind,
        verification,
    })
}

pub(super) fn decode_create(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r12 != 0 || frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::CreateTask {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        intent: IntentId::new(frame.r11),
    })
}

pub(super) fn decode_delegate(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 || frame.r12 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::DelegateTask {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        delegated_task: TaskId::new(frame.r11),
        target: AgentId::new(frame.r12),
    })
}

const fn decode_kind(code: u64) -> Option<IntentKind> {
    match code {
        AGENT_CALL_INTENT_OBSERVE => Some(IntentKind::Observe),
        AGENT_CALL_INTENT_ACT => Some(IntentKind::Act),
        AGENT_CALL_INTENT_VERIFY => Some(IntentKind::Verify),
        AGENT_CALL_INTENT_CHECKPOINT => Some(IntentKind::Checkpoint),
        AGENT_CALL_INTENT_ROLLBACK => Some(IntentKind::Rollback),
        _ => None,
    }
}

const fn decode_verification(code: u64) -> Option<VerificationRequirement> {
    match code {
        AGENT_CALL_VERIFICATION_OPTIONAL => Some(VerificationRequirement::Optional),
        AGENT_CALL_VERIFICATION_REQUIRED => Some(VerificationRequirement::Required),
        _ => None,
    }
}
