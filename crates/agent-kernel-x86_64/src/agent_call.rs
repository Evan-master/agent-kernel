//! Native x86_64 Agent Call ABI register contracts.
//!
//! This architecture-library module decodes bounded call requests from an
//! already-captured privilege frame and encodes read-only context replies. It
//! performs no privileged operation and trusts identity only from an explicit
//! scheduler-owned `AgentCallContext`.

mod context;

use agent_kernel_core::{AgentId, AgentImageId, TaskId, TaskResult};

use crate::context::PrivilegeInterruptStackFrame;

pub use context::AgentCallContext;

pub const AGENT_CALL_ABI_MAGIC: u64 = u64::from_le_bytes(*b"AGNTCALL");
pub const AGENT_CALL_ABI_VERSION: u64 = 1;
pub const AGENT_CALL_DESCRIBE_CONTEXT: u64 = 1;
pub const AGENT_CALL_YIELD: u64 = 2;
pub const AGENT_CALL_COMPLETE_TASK: u64 = 3;
pub const AGENT_CALL_SUBMIT_TASK_RESULT: u64 = 4;
pub const AGENT_CALL_INSPECT_TASK_RESULT: u64 = 5;
pub const AGENT_CALL_VERIFY_TASK: u64 = 6;
pub const AGENT_CALL_STATUS_OK: u64 = 0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallOperation {
    DescribeContext,
    Yield,
    CompleteTask,
    SubmitTaskResult,
    InspectTaskResult,
    VerifyTask,
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
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok((agent, task, image, nonce, TaskId::new(frame.r10)))
}
