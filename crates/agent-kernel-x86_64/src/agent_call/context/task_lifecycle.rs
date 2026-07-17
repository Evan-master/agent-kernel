//! Canonical success replies for Intent and Task lifecycle Agent Calls.
//!
//! This ABI-layer child returns validated kernel-issued handles, preserves the
//! scheduler-owned identity, and clears every unrelated payload register.

use agent_kernel_core::{AgentId, CapabilityId, IntentId, TaskId};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_CREATE_TASK, AGENT_CALL_DECLARE_INTENT,
        AGENT_CALL_DELEGATE_TASK,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_intent_declared_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        intent: IntentId,
    ) -> Result<(), AgentCallDecodeError> {
        if intent.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_DECLARE_INTENT)?;
        frame.r10 = intent.raw();
        Ok(())
    }

    pub fn encode_task_created_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        task: TaskId,
    ) -> Result<(), AgentCallDecodeError> {
        if task.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_CREATE_TASK)?;
        frame.r10 = task.raw();
        Ok(())
    }

    pub fn encode_task_delegated_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        task: TaskId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Result<(), AgentCallDecodeError> {
        if task.raw() == 0 || capability.raw() == 0 || target.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_DELEGATE_TASK)?;
        frame.r10 = task.raw();
        frame.r11 = capability.raw();
        frame.r12 = target.raw();
        Ok(())
    }
}
