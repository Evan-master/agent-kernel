//! Strict register decoding for managed Agent lifecycle calls.
//!
//! This x86 ABI module validates fixed register layouts and converts raw ids
//! into typed requests. Core authorization and lifecycle safety remain inside
//! the kernel facade and deterministic core.

use agent_kernel_core::{AgentId, CapabilityId, ResourceId};

use super::{decode_context_payload, AgentCallDecodeError, AgentCallOperation, AgentCallRequest};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_register(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 || frame.r12 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::RegisterManagedAgent {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        resource: ResourceId::new(frame.r11),
        target: AgentId::new(frame.r12),
    })
}

pub(super) fn decode_lifecycle(
    frame: &PrivilegeInterruptStackFrame,
    operation: AgentCallOperation,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r12 != 0 || frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let authority = CapabilityId::new(frame.r10);
    let target = AgentId::new(frame.r11);
    match operation {
        AgentCallOperation::SuspendManagedAgent => Ok(AgentCallRequest::SuspendManagedAgent {
            agent,
            task,
            image,
            nonce,
            authority,
            target,
        }),
        AgentCallOperation::ResumeManagedAgent => Ok(AgentCallRequest::ResumeManagedAgent {
            agent,
            task,
            image,
            nonce,
            authority,
            target,
        }),
        AgentCallOperation::RetireManagedAgent => Ok(AgentCallRequest::RetireManagedAgent {
            agent,
            task,
            image,
            nonce,
            authority,
            target,
        }),
        _ => Err(AgentCallDecodeError::UnsupportedOperation),
    }
}
