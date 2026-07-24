//! Register decoders for two-stage native durable archive calls.
//!
//! This architecture-library child validates fixed register envelopes only.
//! Scheduler identity authentication and private call-data access remain at
//! the privileged runtime boundary.

use agent_kernel_core::CapabilityId;

use super::{
    decode_context_payload, ensure_extended_reserved_zero, AgentCallDecodeError, AgentCallRequest,
};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_prepare(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 || frame.r12 == 0 || frame.r13 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::PrepareDurableArchive {
        agent,
        task,
        image,
        nonce,
        archive_authority: CapabilityId::new(frame.r10),
        storage_authority: CapabilityId::new(frame.r11),
        through_sequence: frame.r12,
        generation: frame.r13,
    })
}

pub(super) fn decode_commit(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r11 != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    ensure_extended_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::CommitDurableArchiveFromMemory {
        agent,
        task,
        image,
        nonce,
        generation: frame.r10,
    })
}

pub(super) fn decode_sign(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r11 != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    ensure_extended_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::SignDurableArchive {
        agent,
        task,
        image,
        nonce,
        generation: frame.r10,
    })
}
