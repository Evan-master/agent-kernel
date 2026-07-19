//! Canonical reply encoding for retired-Resource Capability cleanup revocation.

use agent_kernel_core::{CapabilityId, ResourceId};

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_REVOKE_CAPABILITY_FOR_CLEANUP},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_capability_cleanup_revocation_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        target: CapabilityId,
        resource: ResourceId,
    ) -> Result<(), AgentCallDecodeError> {
        if target.raw() == 0 || resource.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_REVOKE_CAPABILITY_FOR_CLEANUP)?;
        frame.r10 = target.raw();
        frame.r11 = resource.raw();
        Ok(())
    }
}
