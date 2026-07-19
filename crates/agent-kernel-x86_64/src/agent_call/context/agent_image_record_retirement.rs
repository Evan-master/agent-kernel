//! Canonical reply encoding for Agent Image record retirement.

use agent_kernel_core::{AgentId, AgentImageId, ResourceId};

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_RETIRE_AGENT_IMAGE_RECORD},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_agent_image_record_retirement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        target: AgentImageId,
        resource: ResourceId,
        owner: AgentId,
    ) -> Result<(), AgentCallDecodeError> {
        if target.raw() == 0 || resource.raw() == 0 || owner.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RETIRE_AGENT_IMAGE_RECORD)?;
        frame.r10 = target.raw();
        frame.r11 = resource.raw();
        frame.r12 = owner.raw();
        Ok(())
    }
}
