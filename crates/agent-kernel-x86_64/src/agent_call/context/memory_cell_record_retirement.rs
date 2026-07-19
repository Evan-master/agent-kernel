//! Canonical register reply for terminal MemoryCell record retirement.

use agent_kernel_core::MemoryCellRecord;

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_RETIRE_MEMORY_CELL_RECORD},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_memory_cell_record_retirement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: MemoryCellRecord,
    ) -> Result<(), AgentCallDecodeError> {
        if record.id.raw() == 0
            || record.resource.raw() == 0
            || record.creator.raw() == 0
            || record.last_writer.raw() == 0
            || record.revision == 0
        {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RETIRE_MEMORY_CELL_RECORD)?;
        frame.r10 = record.id.raw();
        frame.r11 = record.resource.raw();
        frame.r12 = record.revision;
        frame.r13 = record.value.words[0];
        frame.r14 = record.value.words[1];
        frame.r15 = record.value.words[2];
        frame.rbp = record.value.words[3];
        Ok(())
    }
}
