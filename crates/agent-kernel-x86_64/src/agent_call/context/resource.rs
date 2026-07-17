//! Canonical success replies for resource lifecycle Agent Calls.
//!
//! This ABI-layer child writes only non-zero kernel-issued Resource and
//! Capability handles while preserving the scheduler-owned call identity.

use agent_kernel_core::{CapabilityId, ResourceCreateOutcome, ResourceId};

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_CREATE_RESOURCE, AGENT_CALL_RETIRE_RESOURCE},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_resource_created_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        outcome: ResourceCreateOutcome,
    ) -> Result<(), AgentCallDecodeError> {
        if outcome.resource.raw() == 0 || outcome.capability.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_CREATE_RESOURCE)?;
        frame.r10 = outcome.resource.raw();
        frame.r11 = outcome.capability.raw();
        Ok(())
    }

    pub fn encode_resource_retired_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        resource: ResourceId,
        capability: CapabilityId,
    ) -> Result<(), AgentCallDecodeError> {
        if resource.raw() == 0 || capability.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RETIRE_RESOURCE)?;
        frame.r10 = resource.raw();
        frame.r11 = capability.raw();
        Ok(())
    }
}
