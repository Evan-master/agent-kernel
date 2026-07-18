//! Scheduler-owned identity and authority for one Agent Call context.
//!
//! Replies expose only Agent, Task, Image, and nonce. The delegated capability
//! remains private to trusted kernel code and participates in context equality.

mod agent_management;
mod authentication;
mod capability;
mod mailbox;
mod memory_page;
mod memory_region;
mod resource;
mod runtime_admission;
mod task_lifecycle;

use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, TaskId, TaskResult};

use super::{
    AgentCallDecodeError, AgentCallRequest, AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION,
    AGENT_CALL_DESCRIBE_CONTEXT, AGENT_CALL_INSPECT_TASK_RESULT, AGENT_CALL_STATUS_OK,
    AGENT_CALL_SUBMIT_TASK_RESULT, AGENT_CALL_VERIFY_TASK, AGENT_CALL_YIELD,
};
use crate::context::PrivilegeInterruptStackFrame;

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
        self.encode_reply(frame, nonce, AGENT_CALL_DESCRIBE_CONTEXT)
    }

    pub fn encode_task_result_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_reply(frame, nonce, AGENT_CALL_SUBMIT_TASK_RESULT)
    }

    pub fn encode_yield_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_reply(frame, nonce, AGENT_CALL_YIELD)
    }

    pub fn encode_task_result_inspection_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        result: TaskResult,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_reply(frame, nonce, AGENT_CALL_INSPECT_TASK_RESULT)?;
        frame.r10 = u64::from(result.code);
        frame.r11 = result.value;
        Ok(())
    }

    pub fn encode_task_verification_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_reply(frame, nonce, AGENT_CALL_VERIFY_TASK)
    }

    pub fn matches_yield(self, request: AgentCallRequest, expected_nonce: u64) -> bool {
        matches!(
            request,
            AgentCallRequest::Yield {
                agent,
                task,
                image,
                nonce,
            } if self.matches_identity(agent, task, image, nonce, expected_nonce)
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
            } if self.matches_identity(agent, task, image, nonce, expected_nonce)
        )
    }

    pub fn match_task_result(
        self,
        request: AgentCallRequest,
        expected_nonce: u64,
    ) -> Option<TaskResult> {
        match request {
            AgentCallRequest::SubmitTaskResult {
                agent,
                task,
                image,
                nonce,
                result,
            } if self.matches_identity(agent, task, image, nonce, expected_nonce) => Some(result),
            _ => None,
        }
    }

    pub fn matches_task_result_inspection(
        self,
        request: AgentCallRequest,
        expected_nonce: u64,
        expected_target: TaskId,
    ) -> bool {
        matches!(
            request,
            AgentCallRequest::InspectTaskResult {
                agent,
                task,
                image,
                nonce,
                target_task,
            } if target_task == expected_target
                && self.matches_identity(agent, task, image, nonce, expected_nonce)
        )
    }

    pub fn matches_task_verification(
        self,
        request: AgentCallRequest,
        expected_nonce: u64,
        expected_target: TaskId,
    ) -> bool {
        matches!(
            request,
            AgentCallRequest::VerifyTask {
                agent,
                task,
                image,
                nonce,
                target_task,
            } if target_task == expected_target
                && self.matches_identity(agent, task, image, nonce, expected_nonce)
        )
    }

    pub const fn capability(self) -> CapabilityId {
        self.capability
    }

    pub const fn agent(self) -> AgentId {
        self.agent
    }

    pub const fn task(self) -> TaskId {
        self.task
    }

    pub const fn image(self) -> AgentImageId {
        self.image
    }

    fn encode_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        operation: u64,
    ) -> Result<(), AgentCallDecodeError> {
        if nonce == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        frame.rax = AGENT_CALL_ABI_MAGIC;
        frame.rbx = AGENT_CALL_ABI_VERSION;
        frame.rcx = AGENT_CALL_STATUS_OK;
        frame.rdx = operation;
        frame.rsi = self.agent.raw();
        frame.rdi = self.task.raw();
        frame.r8 = self.image.raw();
        frame.r9 = nonce;
        frame.r10 = 0;
        frame.r11 = 0;
        frame.r12 = 0;
        frame.r13 = 0;
        frame.r14 = 0;
        frame.r15 = 0;
        frame.rbp = 0;
        Ok(())
    }

    const fn matches_identity(
        self,
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        expected_nonce: u64,
    ) -> bool {
        agent.raw() == self.agent.raw()
            && task.raw() == self.task.raw()
            && image.raw() == self.image.raw()
            && nonce == expected_nonce
            && expected_nonce != 0
    }
}
