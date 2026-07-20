//! Strict register contracts for native Namespace Agent Calls.

use agent_kernel_core::{
    AgentId, CapabilityId, MemoryCellId, MessageId, NamespaceEntryId, NamespaceKey,
    NamespaceObject, NamespacePathSegment, ResourceId, TaskId,
};

use super::{decode_context_payload, AgentCallDecodeError, AgentCallRequest};
use crate::{
    context::PrivilegeInterruptStackFrame, namespace_path_buffer::NAMESPACE_PATH_BUFFER_BYTES,
};

const OBJECT_TAG_MASK: u64 = 0b111;
const MAX_OBJECT_ID: u64 = u64::MAX >> 3;

pub const fn encode_namespace_object(object: NamespaceObject) -> Option<u64> {
    let (id, tag) = match object {
        NamespaceObject::Agent(id) => (id.raw(), 1),
        NamespaceObject::Resource(id) => (id.raw(), 2),
        NamespaceObject::Task(id) => (id.raw(), 3),
        NamespaceObject::Message(id) => (id.raw(), 4),
        NamespaceObject::MemoryCell(id) => (id.raw(), 5),
        NamespaceObject::Mount(id) => (id.raw(), 6),
    };
    if id == 0 || id > MAX_OBJECT_ID {
        None
    } else {
        Some((id << 3) | tag)
    }
}

pub const fn decode_namespace_object(word: u64) -> Option<NamespaceObject> {
    let id = word >> 3;
    if id == 0 {
        return None;
    }
    match word & OBJECT_TAG_MASK {
        1 => Some(NamespaceObject::Agent(AgentId::new(id))),
        2 => Some(NamespaceObject::Resource(ResourceId::new(id))),
        3 => Some(NamespaceObject::Task(TaskId::new(id))),
        4 => Some(NamespaceObject::Message(MessageId::new(id))),
        5 => Some(NamespaceObject::MemoryCell(MemoryCellId::new(id))),
        6 => Some(NamespaceObject::Mount(ResourceId::new(id))),
        _ => None,
    }
}

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
