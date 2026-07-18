//! Admitted context construction and canonical Runtime Admission replies.

use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, RuntimeAdmissionId, TaskId};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_DISCOVER_RUNTIME_ADMISSION,
        AGENT_CALL_REQUEST_RUNTIME_ADMISSION,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub const fn new_admitted(
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        capability: CapabilityId,
        requester: AgentId,
    ) -> Option<Self> {
        if agent.raw() == 0
            || task.raw() == 0
            || image.raw() == 0
            || capability.raw() == 0
            || requester.raw() == 0
        {
            return None;
        }
        Some(Self {
            agent,
            task,
            image,
            capability,
            runtime_admission_requester: Some(requester),
        })
    }

    pub const fn runtime_admission_requester(self) -> Option<AgentId> {
        self.runtime_admission_requester
    }

    pub fn encode_runtime_admission_discovery_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
    ) -> Result<(), AgentCallDecodeError> {
        let requester = self
            .runtime_admission_requester()
            .ok_or(AgentCallDecodeError::RuntimeAdmissionContextUnavailable)?;
        self.encode_reply(frame, nonce, AGENT_CALL_DISCOVER_RUNTIME_ADMISSION)?;
        frame.r10 = requester.raw();
        Ok(())
    }

    pub fn encode_runtime_admission_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        admission: RuntimeAdmissionId,
        target: AgentId,
        target_task: TaskId,
    ) -> Result<(), AgentCallDecodeError> {
        if admission.raw() == 0 || target.raw() == 0 || target_task.raw() == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_REQUEST_RUNTIME_ADMISSION)?;
        frame.r10 = admission.raw();
        frame.r11 = target.raw();
        frame.r12 = target_task.raw();
        Ok(())
    }
}
