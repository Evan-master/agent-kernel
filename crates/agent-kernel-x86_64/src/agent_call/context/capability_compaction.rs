//! Canonical reply encoding for Capability Store compaction.

use agent_kernel_core::CapabilityId;

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_COMPACT_CAPABILITY},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_capability_compaction_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        capability: CapabilityId,
    ) -> Result<(), AgentCallDecodeError> {
        if capability.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_COMPACT_CAPABILITY)?;
        frame.r10 = capability.raw();
        Ok(())
    }
}
