//! Canonical success replies for capability lifecycle Agent Calls.
//!
//! This ABI-layer child returns only validated kernel-issued handles while
//! preserving scheduler-owned identity and clearing every unrelated register.

use agent_kernel_core::CapabilityId;

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_DERIVE_CAPABILITY, AGENT_CALL_REVOKE_DERIVED_CAPABILITY,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_capability_derived_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        capability: CapabilityId,
    ) -> Result<(), AgentCallDecodeError> {
        if capability.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_DERIVE_CAPABILITY)?;
        frame.r10 = capability.raw();
        Ok(())
    }

    pub fn encode_capability_revoked_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        source: CapabilityId,
        target: CapabilityId,
    ) -> Result<(), AgentCallDecodeError> {
        if source.raw() == 0 || target.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_REVOKE_DERIVED_CAPABILITY)?;
        frame.r10 = target.raw();
        frame.r11 = source.raw();
        Ok(())
    }
}
