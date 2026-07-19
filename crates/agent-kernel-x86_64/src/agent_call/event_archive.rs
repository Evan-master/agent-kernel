//! Strict register decoding for Event archive commit calls.

use agent_kernel_core::CapabilityId;

use super::{decode_context_payload, AgentCallDecodeError, AgentCallRequest};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r12 != 0 || frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::ArchiveEvents {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        through_sequence: frame.r11,
    })
}
