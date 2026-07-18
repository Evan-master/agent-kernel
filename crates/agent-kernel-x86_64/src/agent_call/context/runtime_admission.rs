//! Canonical reply for a kernel-issued runtime admission request identity.

use agent_kernel_core::{AgentId, RuntimeAdmissionId, TaskId};

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_REQUEST_RUNTIME_ADMISSION},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
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
