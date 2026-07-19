//! Canonical reply encoding for Agent Record retirement.

use agent_kernel_core::{AgentId, ResourceId};

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_RETIRE_AGENT_RECORD},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_agent_record_retirement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        target: AgentId,
        management_resource: ResourceId,
        retired_floor: AgentId,
    ) -> Result<(), AgentCallDecodeError> {
        if target.raw() == 0 || management_resource.raw() == 0 || retired_floor.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RETIRE_AGENT_RECORD)?;
        frame.r10 = target.raw();
        frame.r11 = management_resource.raw();
        frame.r12 = retired_floor.raw();
        Ok(())
    }
}
