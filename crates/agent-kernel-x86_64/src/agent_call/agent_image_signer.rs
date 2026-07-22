//! Register decoder for typed-memory Agent image signer rotation.
//!
//! This architecture-library child validates only the fixed register envelope.
//! Scheduled identity authentication and call-data snapshotting occur later at
//! the privileged runtime boundary.

use crate::context::PrivilegeInterruptStackFrame;

use super::{
    decode_context_payload, ensure_extended_reserved_zero, AgentCallDecodeError, AgentCallRequest,
};

pub(super) fn decode_rotation(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    if frame.r11 != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    ensure_extended_reserved_zero(frame)?;
    Ok(AgentCallRequest::RotateAgentImageSignerFromMemory {
        agent,
        task,
        image,
        nonce,
        generation: frame.r10,
    })
}
