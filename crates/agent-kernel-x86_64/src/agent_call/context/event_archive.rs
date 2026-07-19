//! Canonical bounded reply encoding for Event archive checkpoints.

use agent_kernel_core::EventArchiveDigest;

use super::AgentCallContext;
use crate::{
    agent_call::{AgentCallDecodeError, AGENT_CALL_ARCHIVE_EVENTS},
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_event_archive_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        first_sequence: u64,
        through_sequence: u64,
        count: usize,
        digest: EventArchiveDigest,
    ) -> Result<(), AgentCallDecodeError> {
        let count = u64::try_from(count).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
        if first_sequence == 0
            || through_sequence < first_sequence
            || count == 0
            || through_sequence - first_sequence + 1 != count
        {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_ARCHIVE_EVENTS)?;
        frame.r10 = first_sequence;
        frame.r11 = through_sequence;
        frame.r12 = count;
        let words = digest.words_le();
        frame.r13 = words[0];
        frame.r14 = words[1];
        frame.r15 = words[2];
        frame.rbp = words[3];
        Ok(())
    }
}
