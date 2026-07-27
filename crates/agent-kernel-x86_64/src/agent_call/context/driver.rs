//! Canonical replies for native Driver Agent Calls.
//!
//! Replies expose semantic records only. Capability and endpoint coordinates
//! stay private to trusted kernel and architecture adapters.

use agent_kernel_core::{
    DeviceEventId, DeviceEventKind, DeviceEventPayload, DriverBindingId, DriverCommandId,
    DriverCommandResult, ResourceId,
};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_ACKNOWLEDGE_DEVICE_EVENT, AGENT_CALL_EVENT_DATA_READY,
        AGENT_CALL_EVENT_FAULT, AGENT_CALL_EVENT_INTERRUPT, AGENT_CALL_EVENT_STATE_CHANGED,
        AGENT_CALL_INSPECT_DRIVER_INVOCATION, AGENT_CALL_SUBMIT_DRIVER_COMMAND,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    #[allow(clippy::too_many_arguments)]
    pub fn encode_driver_invocation_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        event: DeviceEventId,
        resource: ResourceId,
        binding: DriverBindingId,
        kind: DeviceEventKind,
        payload: DeviceEventPayload,
    ) -> Result<(), AgentCallDecodeError> {
        if self.driver_invocation().is_none()
            || event.raw() == 0
            || resource.raw() == 0
            || binding.raw() == 0
        {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_INSPECT_DRIVER_INVOCATION)?;
        frame.r10 = event.raw();
        frame.r11 = resource.raw();
        frame.r12 = binding.raw();
        frame.r13 = encode_event_kind(kind);
        frame.r14 = u64::from(payload.code);
        frame.r15 = payload.value;
        Ok(())
    }

    pub fn encode_device_event_acknowledgement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        event: DeviceEventId,
    ) -> Result<(), AgentCallDecodeError> {
        if self.driver_invocation().is_none() || event.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_ACKNOWLEDGE_DEVICE_EVENT)?;
        frame.r10 = event.raw();
        Ok(())
    }

    pub fn encode_driver_command_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        command: DriverCommandId,
        result: DriverCommandResult,
    ) -> Result<(), AgentCallDecodeError> {
        if self.driver_invocation().is_none() || command.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_SUBMIT_DRIVER_COMMAND)?;
        frame.r10 = command.raw();
        frame.r11 = u64::from(result.code);
        frame.r12 = result.value;
        Ok(())
    }
}

const fn encode_event_kind(kind: DeviceEventKind) -> u64 {
    match kind {
        DeviceEventKind::Interrupt => AGENT_CALL_EVENT_INTERRUPT,
        DeviceEventKind::DataReady => AGENT_CALL_EVENT_DATA_READY,
        DeviceEventKind::Fault => AGENT_CALL_EVENT_FAULT,
        DeviceEventKind::StateChanged => AGENT_CALL_EVENT_STATE_CHANGED,
    }
}
