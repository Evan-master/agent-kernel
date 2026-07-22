//! Canonical reply encoding for one committed signer rotation.

use super::{AgentCallContext, AgentCallDecodeError};
use crate::{
    agent_call::AGENT_CALL_ROTATE_AGENT_IMAGE_SIGNER, context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_agent_image_signer_rotation_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        policy_generation: u64,
        signer_count: usize,
    ) -> Result<(), AgentCallDecodeError> {
        if policy_generation == 0 || signer_count == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_ROTATE_AGENT_IMAGE_SIGNER)?;
        frame.r10 = policy_generation;
        frame.r11 = signer_count as u64;
        frame.r12 = 2;
        frame.r13 = 2;
        frame.r14 = 1;
        Ok(())
    }
}
