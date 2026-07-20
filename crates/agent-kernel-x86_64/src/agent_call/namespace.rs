//! Strict register contracts for native Namespace Agent Calls.

use agent_kernel_core::{
    CapabilityId, NamespaceEntryId, NamespaceKey, NamespacePathSegment, ResourceId,
};

use super::{decode_context_payload, AgentCallDecodeError, AgentCallRequest};
use crate::{
    context::PrivilegeInterruptStackFrame, namespace_path_buffer::NAMESPACE_PATH_BUFFER_BYTES,
};

pub use crate::namespace_object_wire::{decode_namespace_object, encode_namespace_object};

pub(super) fn decode_bind(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let object = decode_namespace_object(frame.r13).ok_or(AgentCallDecodeError::InvalidPayload)?;
    Ok(AgentCallRequest::BindNamespaceEntry {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        namespace: ResourceId::new(frame.r11),
        key: NamespaceKey::new(frame.r12),
        object,
    })
}

pub(super) fn decode_resolve(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::ResolveNamespaceEntry {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        namespace: ResourceId::new(frame.r11),
        key: NamespaceKey::new(frame.r12),
    })
}

pub(super) fn decode_rebind(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let object = decode_namespace_object(frame.r12).ok_or(AgentCallDecodeError::InvalidPayload)?;
    Ok(AgentCallRequest::RebindNamespaceEntry {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        entry: NamespaceEntryId::new(frame.r11),
        object,
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
    Ok(AgentCallRequest::RetireNamespaceEntry {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        entry: NamespaceEntryId::new(frame.r11),
    })
}

pub(super) fn decode_compare_rebind(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 || frame.r12 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let object = decode_namespace_object(frame.r13).ok_or(AgentCallDecodeError::InvalidPayload)?;
    Ok(AgentCallRequest::CompareAndRebindNamespaceEntry {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        entry: NamespaceEntryId::new(frame.r11),
        expected_revision: frame.r12,
        object,
    })
}

pub(super) fn decode_compare_retire(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 || frame.r12 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::CompareAndRetireNamespaceEntry {
        agent,
        task,
        image,
        nonce,
        authority: CapabilityId::new(frame.r10),
        entry: NamespaceEntryId::new(frame.r11),
        expected_revision: frame.r12,
    })
}

pub(super) fn decode_path(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r12 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    let second = match frame.r11 {
        1 => {
            if frame.r14 != 0 || frame.r15 != 0 {
                return Err(AgentCallDecodeError::ReservedNotZero);
            }
            None
        }
        2 => {
            if frame.r14 == 0 {
                return Err(AgentCallDecodeError::InvalidPayload);
            }
            Some(NamespacePathSegment::new(
                CapabilityId::new(frame.r14),
                NamespaceKey::new(frame.r15),
            ))
        }
        _ => return Err(AgentCallDecodeError::InvalidPayload),
    };
    Ok(AgentCallRequest::ResolveNamespacePath {
        agent,
        task,
        image,
        nonce,
        root: ResourceId::new(frame.r10),
        first: NamespacePathSegment::new(
            CapabilityId::new(frame.r12),
            NamespaceKey::new(frame.r13),
        ),
        second,
    })
}

pub(super) fn decode_memory_path(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r13 != 0 || frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 || frame.r11 == 0 || frame.r12 != NAMESPACE_PATH_BUFFER_BYTES as u64 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::ResolveNamespacePathFromMemory {
        agent,
        task,
        image,
        nonce,
        root: ResourceId::new(frame.r10),
        generation: frame.r11,
    })
}

pub(super) fn decode_memory_path_rebind(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    if frame.r11 != 0
        || frame.r12 != 0
        || frame.r13 != 0
        || frame.r14 != 0
        || frame.r15 != 0
        || frame.rbp != 0
    {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok(AgentCallRequest::CompareAndRebindNamespacePathFromMemory {
        agent,
        task,
        image,
        nonce,
        generation: frame.r10,
    })
}
