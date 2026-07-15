//! Native x86_64 Agent Call ABI register contracts.
//!
//! This architecture-library module decodes bounded call requests from an
//! already-captured privilege frame and encodes read-only context replies. It
//! performs no privileged operation and trusts identity only from an explicit
//! scheduler-owned `AgentCallContext`.

use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, TaskId};

use crate::context::PrivilegeInterruptStackFrame;

pub const AGENT_CALL_ABI_MAGIC: u64 = u64::from_le_bytes(*b"AGNTCALL");
pub const AGENT_CALL_ABI_VERSION: u64 = 1;
pub const AGENT_CALL_DESCRIBE_CONTEXT: u64 = 1;
pub const AGENT_CALL_YIELD: u64 = 2;
pub const AGENT_CALL_COMPLETE_TASK: u64 = 3;
pub const AGENT_CALL_STATUS_OK: u64 = 0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallOperation {
    DescribeContext,
    Yield,
    CompleteTask,
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
            _ => return Err(AgentCallDecodeError::UnsupportedOperation),
        };
        if frame.rdx != 0 {
            return Err(AgentCallDecodeError::UnsupportedFlags);
        }
        if frame.r10 != 0 || frame.r11 != 0 {
            return Err(AgentCallDecodeError::ReservedNotZero);
        }

        match operation {
            AgentCallOperation::DescribeContext => {
                if frame.rsi == 0 || frame.rdi != 0 || frame.r8 != 0 || frame.r9 != 0 {
                    return Err(AgentCallDecodeError::InvalidPayload);
                }
                Ok(Self::DescribeContext { nonce: frame.rsi })
            }
            AgentCallOperation::Yield => {
                if frame.rsi == 0 || frame.rdi == 0 || frame.r8 == 0 || frame.r9 == 0 {
                    return Err(AgentCallDecodeError::InvalidPayload);
                }
                Ok(Self::Yield {
                    agent: AgentId::new(frame.rsi),
                    task: TaskId::new(frame.rdi),
                    image: AgentImageId::new(frame.r8),
                    nonce: frame.r9,
                })
            }
            AgentCallOperation::CompleteTask => {
                if frame.rsi == 0 || frame.rdi == 0 || frame.r8 == 0 || frame.r9 == 0 {
                    return Err(AgentCallDecodeError::InvalidPayload);
                }
                Ok(Self::CompleteTask {
                    agent: AgentId::new(frame.rsi),
                    task: TaskId::new(frame.rdi),
                    image: AgentImageId::new(frame.r8),
                    nonce: frame.r9,
                })
            }
        }
    }

    pub const fn operation(self) -> AgentCallOperation {
        match self {
            Self::DescribeContext { .. } => AgentCallOperation::DescribeContext,
            Self::Yield { .. } => AgentCallOperation::Yield,
            Self::CompleteTask { .. } => AgentCallOperation::CompleteTask,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentCallContext {
    agent: AgentId,
    task: TaskId,
    image: AgentImageId,
    capability: CapabilityId,
}

impl AgentCallContext {
    pub const fn new(
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        capability: CapabilityId,
    ) -> Option<Self> {
        if agent.raw() == 0 || task.raw() == 0 || image.raw() == 0 || capability.raw() == 0 {
            return None;
        }
        Some(Self {
            agent,
            task,
            image,
            capability,
        })
    }

    pub fn encode_describe_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
    ) -> Result<(), AgentCallDecodeError> {
        if nonce == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        frame.rax = AGENT_CALL_ABI_MAGIC;
        frame.rbx = AGENT_CALL_ABI_VERSION;
        frame.rcx = AGENT_CALL_STATUS_OK;
        frame.rdx = AGENT_CALL_DESCRIBE_CONTEXT;
        frame.rsi = self.agent.raw();
        frame.rdi = self.task.raw();
        frame.r8 = self.image.raw();
        frame.r9 = nonce;
        frame.r10 = 0;
        frame.r11 = 0;
        Ok(())
    }

    pub fn matches_yield(self, request: AgentCallRequest, expected_nonce: u64) -> bool {
        matches!(
            request,
            AgentCallRequest::Yield {
                agent,
                task,
                image,
                nonce,
            } if agent == self.agent
                && task == self.task
                && image == self.image
                && nonce == expected_nonce
                && expected_nonce != 0
        )
    }

    pub fn matches_completion(self, request: AgentCallRequest, expected_nonce: u64) -> bool {
        matches!(
            request,
            AgentCallRequest::CompleteTask {
                agent,
                task,
                image,
                nonce,
            } if agent == self.agent
                && task == self.task
                && image == self.image
                && nonce == expected_nonce
                && expected_nonce != 0
        )
    }

    pub const fn capability(self) -> CapabilityId {
        self.capability
    }
}
