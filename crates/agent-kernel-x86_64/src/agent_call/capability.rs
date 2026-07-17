//! Strict register decoding for capability lifecycle Agent Calls.
//!
//! This x86 ABI child accepts only non-zero kernel handles and the canonical
//! six-bit operation encoding. It performs no authority decision; the trusted
//! facade and core validate source ownership, scope, attenuation, and lineage.

use agent_kernel_core::{AgentId, CapabilityId, OperationSet};

use super::{decode_context_payload, AgentCallDecodeError, AgentCallRequest};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_derive(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let bits = u16::try_from(frame.r12).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
    let operations = OperationSet::from_bits(bits).ok_or(AgentCallDecodeError::InvalidPayload)?;
    if operations == OperationSet::empty() {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::DeriveCapability {
        agent,
        task,
        image,
        nonce,
        source: CapabilityId::new(frame.r10),
        target: AgentId::new(frame.r11),
        operations,
    })
}

pub(super) fn decode_revoke(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r12 != 0 || frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::RevokeDerivedCapability {
        agent,
        task,
        image,
        nonce,
        source: CapabilityId::new(frame.r10),
        target: CapabilityId::new(frame.r11),
    })
}
