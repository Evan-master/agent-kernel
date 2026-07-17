//! Native x86_64 Agent Call ABI register contracts.
//!
//! This architecture-library module decodes bounded call requests from an
//! already-captured privilege frame and encodes read-only context replies. It
//! performs no privileged operation and trusts identity only from an explicit
//! scheduler-owned `AgentCallContext`.

mod capability;
mod context;
mod mailbox;
mod resource;
mod transcript;

use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, MessageId, MessageKind, MessagePayload, OperationSet,
    ResourceId, ResourceKind, TaskId, TaskResult,
};

use crate::context::PrivilegeInterruptStackFrame;

pub use context::AgentCallContext;
pub use transcript::{AgentCallTranscript, AgentCallTranscriptError};

pub const AGENT_CALL_ABI_MAGIC: u64 = u64::from_le_bytes(*b"AGNTCALL");
pub const AGENT_CALL_ABI_VERSION: u64 = 1;
pub const AGENT_CALL_DESCRIBE_CONTEXT: u64 = 1;
pub const AGENT_CALL_YIELD: u64 = 2;
pub const AGENT_CALL_COMPLETE_TASK: u64 = 3;
pub const AGENT_CALL_SUBMIT_TASK_RESULT: u64 = 4;
pub const AGENT_CALL_INSPECT_TASK_RESULT: u64 = 5;
pub const AGENT_CALL_VERIFY_TASK: u64 = 6;
pub const AGENT_CALL_SEND_MESSAGE: u64 = 7;
pub const AGENT_CALL_RECEIVE_MESSAGE: u64 = 8;
pub const AGENT_CALL_ACKNOWLEDGE_MESSAGE: u64 = 9;
pub const AGENT_CALL_CREATE_RESOURCE: u64 = 10;
pub const AGENT_CALL_RETIRE_RESOURCE: u64 = 11;
pub const AGENT_CALL_DERIVE_CAPABILITY: u64 = 12;
pub const AGENT_CALL_REVOKE_DERIVED_CAPABILITY: u64 = 13;
pub const AGENT_CALL_MESSAGE_NOTIFY: u64 = 1;
pub const AGENT_CALL_MESSAGE_REQUEST: u64 = 2;
pub const AGENT_CALL_MESSAGE_RESPONSE: u64 = 3;
pub const AGENT_CALL_MESSAGE_FAULT: u64 = 4;
pub const AGENT_CALL_RESOURCE_WORKSPACE: u64 = 1;
pub const AGENT_CALL_RESOURCE_MEMORY: u64 = 2;
pub const AGENT_CALL_RESOURCE_SERVICE: u64 = 3;
pub const AGENT_CALL_RESOURCE_NETWORK: u64 = 4;
pub const AGENT_CALL_RESOURCE_DEVICE: u64 = 5;
pub const AGENT_CALL_STATUS_OK: u64 = 0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallOperation {
    DescribeContext,
    Yield,
    CompleteTask,
    SubmitTaskResult,
    InspectTaskResult,
    VerifyTask,
    SendMessage,
    ReceiveMessage,
    AcknowledgeMessage,
    CreateResource,
    RetireResource,
    DeriveCapability,
    RevokeDerivedCapability,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallRequest {
    DescribeContext {
        nonce: u64,
    },
    Yield {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
    },
    CompleteTask {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
    },
    SubmitTaskResult {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        result: TaskResult,
    },
    InspectTaskResult {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        target_task: TaskId,
    },
    VerifyTask {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        target_task: TaskId,
    },
    SendMessage {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        recipient: AgentId,
        kind: MessageKind,
        payload: MessagePayload,
    },
    ReceiveMessage {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
    },
    AcknowledgeMessage {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        message: MessageId,
    },
    CreateResource {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        parent: ResourceId,
        kind: ResourceKind,
        operations: OperationSet,
    },
    RetireResource {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        resource: ResourceId,
        capability: CapabilityId,
    },
    DeriveCapability {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        source: CapabilityId,
        target: AgentId,
        operations: OperationSet,
    },
    RevokeDerivedCapability {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        source: CapabilityId,
        target: CapabilityId,
    },
}

impl AgentCallRequest {
    pub fn decode(frame: &PrivilegeInterruptStackFrame) -> Result<Self, AgentCallDecodeError> {
        if frame.rax != AGENT_CALL_ABI_MAGIC {
            return Err(AgentCallDecodeError::InvalidMagic);
        }
        if frame.rbx != AGENT_CALL_ABI_VERSION {
            return Err(AgentCallDecodeError::UnsupportedVersion);
        }
        let operation = match frame.rcx {
            AGENT_CALL_DESCRIBE_CONTEXT => AgentCallOperation::DescribeContext,
            AGENT_CALL_YIELD => AgentCallOperation::Yield,
            AGENT_CALL_COMPLETE_TASK => AgentCallOperation::CompleteTask,
            AGENT_CALL_SUBMIT_TASK_RESULT => AgentCallOperation::SubmitTaskResult,
            AGENT_CALL_INSPECT_TASK_RESULT => AgentCallOperation::InspectTaskResult,
            AGENT_CALL_VERIFY_TASK => AgentCallOperation::VerifyTask,
            AGENT_CALL_SEND_MESSAGE => AgentCallOperation::SendMessage,
            AGENT_CALL_RECEIVE_MESSAGE => AgentCallOperation::ReceiveMessage,
            AGENT_CALL_ACKNOWLEDGE_MESSAGE => AgentCallOperation::AcknowledgeMessage,
            AGENT_CALL_CREATE_RESOURCE => AgentCallOperation::CreateResource,
            AGENT_CALL_RETIRE_RESOURCE => AgentCallOperation::RetireResource,
            AGENT_CALL_DERIVE_CAPABILITY => AgentCallOperation::DeriveCapability,
            AGENT_CALL_REVOKE_DERIVED_CAPABILITY => AgentCallOperation::RevokeDerivedCapability,
            _ => return Err(AgentCallDecodeError::UnsupportedOperation),
        };
        if frame.rdx != 0 {
            return Err(AgentCallDecodeError::UnsupportedFlags);
        }
        match operation {
            AgentCallOperation::DescribeContext => {
                ensure_reserved_zero(frame)?;
                if frame.rsi == 0 || frame.rdi != 0 || frame.r8 != 0 || frame.r9 != 0 {
                    return Err(AgentCallDecodeError::InvalidPayload);
                }
                Ok(Self::DescribeContext { nonce: frame.rsi })
            }
            AgentCallOperation::Yield => {
                ensure_reserved_zero(frame)?;
                let (agent, task, image, nonce) = decode_context_payload(frame)?;
                Ok(Self::Yield {
                    agent,
                    task,
                    image,
                    nonce,
                })
            }
            AgentCallOperation::CompleteTask => {
                ensure_reserved_zero(frame)?;
                let (agent, task, image, nonce) = decode_context_payload(frame)?;
                Ok(Self::CompleteTask {
                    agent,
                    task,
                    image,
                    nonce,
                })
            }
            AgentCallOperation::SubmitTaskResult => {
                ensure_extended_reserved_zero(frame)?;
                let (agent, task, image, nonce) = decode_context_payload(frame)?;
                let code =
                    u16::try_from(frame.r10).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
                Ok(Self::SubmitTaskResult {
                    agent,
                    task,
                    image,
                    nonce,
                    result: TaskResult {
                        code,
                        value: frame.r11,
                    },
                })
            }
            AgentCallOperation::InspectTaskResult => {
                let (agent, task, image, nonce, target_task) = decode_verifier_payload(frame)?;
                Ok(Self::InspectTaskResult {
                    agent,
                    task,
                    image,
                    nonce,
                    target_task,
                })
            }
            AgentCallOperation::VerifyTask => {
                let (agent, task, image, nonce, target_task) = decode_verifier_payload(frame)?;
                Ok(Self::VerifyTask {
                    agent,
                    task,
                    image,
                    nonce,
                    target_task,
                })
            }
            AgentCallOperation::SendMessage => mailbox::decode_send(frame),
            AgentCallOperation::ReceiveMessage => mailbox::decode_receive(frame),
            AgentCallOperation::AcknowledgeMessage => mailbox::decode_acknowledgement(frame),
            AgentCallOperation::CreateResource => resource::decode_create(frame),
            AgentCallOperation::RetireResource => resource::decode_retire(frame),
            AgentCallOperation::DeriveCapability => capability::decode_derive(frame),
            AgentCallOperation::RevokeDerivedCapability => capability::decode_revoke(frame),
        }
    }

    pub const fn operation(self) -> AgentCallOperation {
        match self {
            Self::DescribeContext { .. } => AgentCallOperation::DescribeContext,
            Self::Yield { .. } => AgentCallOperation::Yield,
            Self::CompleteTask { .. } => AgentCallOperation::CompleteTask,
            Self::SubmitTaskResult { .. } => AgentCallOperation::SubmitTaskResult,
            Self::InspectTaskResult { .. } => AgentCallOperation::InspectTaskResult,
            Self::VerifyTask { .. } => AgentCallOperation::VerifyTask,
            Self::SendMessage { .. } => AgentCallOperation::SendMessage,
            Self::ReceiveMessage { .. } => AgentCallOperation::ReceiveMessage,
            Self::AcknowledgeMessage { .. } => AgentCallOperation::AcknowledgeMessage,
            Self::CreateResource { .. } => AgentCallOperation::CreateResource,
            Self::RetireResource { .. } => AgentCallOperation::RetireResource,
            Self::DeriveCapability { .. } => AgentCallOperation::DeriveCapability,
            Self::RevokeDerivedCapability { .. } => AgentCallOperation::RevokeDerivedCapability,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallDecodeError {
    InvalidMagic,
    UnsupportedVersion,
    UnsupportedOperation,
    UnsupportedFlags,
    ReservedNotZero,
    InvalidPayload,
}

fn ensure_reserved_zero(frame: &PrivilegeInterruptStackFrame) -> Result<(), AgentCallDecodeError> {
    if frame.r10 == 0 && frame.r11 == 0 {
        ensure_extended_reserved_zero(frame)
    } else {
        Err(AgentCallDecodeError::ReservedNotZero)
    }
}

fn ensure_extended_reserved_zero(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(), AgentCallDecodeError> {
    if frame.r12 == 0 && frame.r13 == 0 && frame.r14 == 0 && frame.r15 == 0 && frame.rbp == 0 {
        Ok(())
    } else {
        Err(AgentCallDecodeError::ReservedNotZero)
    }
}

fn decode_context_payload(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(AgentId, TaskId, AgentImageId, u64), AgentCallDecodeError> {
    if frame.rsi == 0 || frame.rdi == 0 || frame.r8 == 0 || frame.r9 == 0 {
        Err(AgentCallDecodeError::InvalidPayload)
    } else {
        Ok((
            AgentId::new(frame.rsi),
            TaskId::new(frame.rdi),
            AgentImageId::new(frame.r8),
            frame.r9,
        ))
    }
}

fn decode_verifier_payload(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(AgentId, TaskId, AgentImageId, u64, TaskId), AgentCallDecodeError> {
    if frame.r11 != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    ensure_extended_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok((agent, task, image, nonce, TaskId::new(frame.r10)))
}
