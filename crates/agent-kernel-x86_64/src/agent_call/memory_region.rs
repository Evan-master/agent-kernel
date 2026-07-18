//! Strict register decoding for native runtime memory-region calls.
//!
//! This x86 ABI child accepts typed handles and one bounded page count. The
//! kernel selects addresses, physical frames, flags, and byte length; every
//! extension register outside the operation payload must remain zero.

use agent_kernel_core::{CapabilityId, MemoryCellId, ResourceId};

use super::{
    decode_context_payload, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
    AGENT_CALL_MEMORY_REGION_MAX_PAGES,
};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_allocate(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0
        || frame.r11 == 0
        || frame.r12 == 0
        || frame.r12 > AGENT_CALL_MEMORY_REGION_MAX_PAGES
    {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::AllocateMemoryRegion {
        agent,
        task,
        image,
        nonce,
        capability: CapabilityId::new(frame.r10),
        resource: ResourceId::new(frame.r11),
        page_count: frame.r12,
    })
}

pub(super) fn decode_existing(
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
    let capability = CapabilityId::new(frame.r10);
    let cell = MemoryCellId::new(frame.r11);
    match operation {
        AgentCallOperation::InspectMemoryRegion => Ok(AgentCallRequest::InspectMemoryRegion {
            agent,
            task,
            image,
            nonce,
            capability,
            cell,
        }),
        AgentCallOperation::ReleaseMemoryRegion => Ok(AgentCallRequest::ReleaseMemoryRegion {
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
