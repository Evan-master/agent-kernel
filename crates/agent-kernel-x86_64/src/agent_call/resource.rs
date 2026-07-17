//! Strict register decoding for resource lifecycle Agent Calls.
//!
//! This x86 ABI module converts bounded wire values into AgentOS-native
//! resource types. It rejects legacy-facing kinds, unknown authority bits, and
//! non-zero reserved registers before trusted kernel code can mutate state.

use agent_kernel_core::{CapabilityId, OperationSet, ResourceId, ResourceKind};

use super::{
    decode_context_payload, AgentCallDecodeError, AgentCallRequest, AGENT_CALL_RESOURCE_DEVICE,
    AGENT_CALL_RESOURCE_MEMORY, AGENT_CALL_RESOURCE_NETWORK, AGENT_CALL_RESOURCE_SERVICE,
    AGENT_CALL_RESOURCE_WORKSPACE,
};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode_create(
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
    let bits = u16::try_from(frame.r13).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
    let operations = OperationSet::from_bits(bits).ok_or(AgentCallDecodeError::InvalidPayload)?;
    if operations == OperationSet::empty() {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::CreateResource {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        parent: ResourceId::new(frame.r11),
        kind,
        operations,
    })
}

pub(super) fn decode_retire(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r12 != 0 || frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::RetireResource {
        agent,
        task,
        image,
        nonce,
        resource: ResourceId::new(frame.r10),
        capability: CapabilityId::new(frame.r11),
    })
}

const fn decode_kind(code: u64) -> Option<ResourceKind> {
    match code {
        AGENT_CALL_RESOURCE_WORKSPACE => Some(ResourceKind::Workspace),
        AGENT_CALL_RESOURCE_MEMORY => Some(ResourceKind::Memory),
        AGENT_CALL_RESOURCE_SERVICE => Some(ResourceKind::Service),
        AGENT_CALL_RESOURCE_NETWORK => Some(ResourceKind::Network),
        AGENT_CALL_RESOURCE_DEVICE => Some(ResourceKind::Device),
        _ => None,
    }
}
