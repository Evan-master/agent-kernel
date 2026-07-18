//! Canonical replies for managed Agent lifecycle calls.
//!
//! This x86 ABI module encodes target identity, management Resource, and final
//! status after a successful facade transition. It rejects mismatched
//! operation/status pairs and clears every unrelated payload register.

use agent_kernel_core::{AgentId, AgentStatus, ResourceId};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_AGENT_ACTIVE, AGENT_CALL_AGENT_RETIRED,
        AGENT_CALL_AGENT_SUSPENDED, AGENT_CALL_REGISTER_MANAGED_AGENT,
        AGENT_CALL_RESUME_MANAGED_AGENT, AGENT_CALL_RETIRE_MANAGED_AGENT,
        AGENT_CALL_SUSPEND_MANAGED_AGENT,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_agent_management_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        operation: u64,
        target: AgentId,
        resource: ResourceId,
        status: AgentStatus,
    ) -> Result<(), AgentCallDecodeError> {
        let status_code = match (operation, status) {
            (AGENT_CALL_REGISTER_MANAGED_AGENT, AgentStatus::Active)
            | (AGENT_CALL_RESUME_MANAGED_AGENT, AgentStatus::Active) => AGENT_CALL_AGENT_ACTIVE,
            (AGENT_CALL_SUSPEND_MANAGED_AGENT, AgentStatus::Suspended) => {
                AGENT_CALL_AGENT_SUSPENDED
            }
            (AGENT_CALL_RETIRE_MANAGED_AGENT, AgentStatus::Retired) => AGENT_CALL_AGENT_RETIRED,
            _ => return Err(AgentCallDecodeError::InvalidPayload),
        };
        if target.raw() == 0 || resource.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, operation)?;
        frame.r10 = target.raw();
        frame.r11 = resource.raw();
        frame.r12 = status_code;
        Ok(())
    }
}
