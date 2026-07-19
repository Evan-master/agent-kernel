//! Strict register decoding for runtime admission lifecycle calls.

use agent_kernel_core::{AgentId, CapabilityId, RuntimeAdmissionId, TaskId};

use super::{decode_context_payload, ensure_reserved_zero, AgentCallDecodeError, AgentCallRequest};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_request(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 || frame.r12 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::RequestRuntimeAdmission {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        target: AgentId::new(frame.r11),
        target_task: TaskId::new(frame.r12),
    })
}

pub(super) fn decode_discovery(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    ensure_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    Ok(AgentCallRequest::DiscoverRuntimeAdmission {
        agent,
        task,
        image,
        nonce,
    })
}

pub(super) fn decode_compaction(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r12 != 0 || frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::CompactRuntimeAdmissions {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        through: RuntimeAdmissionId::new(frame.r11),
    })
}
