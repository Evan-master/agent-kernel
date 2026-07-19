//! Canonical reply encoding for Agent Entry retirement.

use agent_kernel_core::AgentId;

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_RETIRE_AGENT_ENTRY},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_agent_entry_retirement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        agent: AgentId,
    ) -> Result<(), AgentCallDecodeError> {
        if agent.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RETIRE_AGENT_ENTRY)?;
        frame.r10 = agent.raw();
        Ok(())
    }
}
