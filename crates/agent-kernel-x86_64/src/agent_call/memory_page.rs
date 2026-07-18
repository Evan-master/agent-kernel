//! Strict register decoding for native runtime memory-page calls.
//!
//! This x86 ABI child accepts only capability and kernel-object handles. Raw
//! addresses, lengths, flags, and physical identities stay kernel-selected;
//! every unused extension register is required to be zero.

use agent_kernel_core::{CapabilityId, MemoryCellId, ResourceId};

use super::{decode_context_payload, AgentCallDecodeError, AgentCallOperation, AgentCallRequest};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_allocate(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    ensure_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::AllocateMemoryPage {
        agent,
        task,
        image,
        nonce,
        capability: CapabilityId::new(frame.r10),
        resource: ResourceId::new(frame.r11),
    })
}

pub(super) fn decode_existing(
    frame: &PrivilegeInterruptStackFrame,
    operation: AgentCallOperation,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    ensure_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let capability = CapabilityId::new(frame.r10);
    let cell = MemoryCellId::new(frame.r11);
    match operation {
        AgentCallOperation::InspectMemoryPage => Ok(AgentCallRequest::InspectMemoryPage {
            agent,
            task,
            image,
            nonce,
            capability,
            cell,
        }),
        AgentCallOperation::ReleaseMemoryPage => Ok(AgentCallRequest::ReleaseMemoryPage {
            agent,
            task,
            image,
            nonce,
            capability,
            cell,
        }),
        _ => Err(AgentCallDecodeError::UnsupportedOperation),
    }
}

fn ensure_reserved_zero(frame: &PrivilegeInterruptStackFrame) -> Result<(), AgentCallDecodeError> {
    if frame.r12 == 0 && frame.r13 == 0 && frame.r14 == 0 && frame.r15 == 0 && frame.rbp == 0 {
        Ok(())
    } else {
        Err(AgentCallDecodeError::ReservedNotZero)
    }
}
