//! Canonical bounded reply encoding for Task Store compaction.

use agent_kernel_core::TaskId;

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_COMPACT_TASKS},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_task_compaction_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        first: TaskId,
        through: TaskId,
        count: usize,
    ) -> Result<(), AgentCallDecodeError> {
        if first.raw() == 0 || through.raw() == 0 || first.raw() > through.raw() || count == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        let count = u64::try_from(count).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
        self.encode_reply(frame, nonce, AGENT_CALL_COMPACT_TASKS)?;
        frame.r10 = first.raw();
        frame.r11 = through.raw();
        frame.r12 = count;
        Ok(())
    }
}
