//! Admitted context construction and canonical Runtime Admission replies.
//!
//! Discovery, request, and compaction replies expose only bounded kernel-owned
//! identities and counts through registers.

use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, RuntimeAdmissionId, TaskId};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_COMPACT_RUNTIME_ADMISSIONS,
        AGENT_CALL_DISCOVER_RUNTIME_ADMISSION, AGENT_CALL_REQUEST_RUNTIME_ADMISSION,
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

    pub fn encode_runtime_admission_compaction_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        first: RuntimeAdmissionId,
        through: RuntimeAdmissionId,
        count: usize,
    ) -> Result<(), AgentCallDecodeError> {
        if first.raw() == 0 || through.raw() == 0 || first.raw() > through.raw() || count == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        let count = u64::try_from(count).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
        self.encode_reply(frame, nonce, AGENT_CALL_COMPACT_RUNTIME_ADMISSIONS)?;
        frame.r10 = first.raw();
        frame.r11 = through.raw();
        frame.r12 = count;
        Ok(())
    }
}
