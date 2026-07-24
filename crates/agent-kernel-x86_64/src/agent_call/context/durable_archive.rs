//! Canonical replies for two-stage durable archive operations.

use agent_kernel_core::{EventArchiveDigest, DURABLE_ARCHIVE_MANIFEST_BYTES};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_COMMIT_DURABLE_ARCHIVE,
        AGENT_CALL_PREPARE_DURABLE_ARCHIVE, AGENT_CALL_SIGN_DURABLE_ARCHIVE,
    },
    context::PrivilegeInterruptStackFrame,
    durable_archive_request::DURABLE_ARCHIVE_REQUEST_BYTES,
};

impl AgentCallContext {
    #[allow(clippy::too_many_arguments)]
    pub fn encode_durable_archive_prepare_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        generation: u64,
        first_sequence: u64,
        through_sequence: u64,
        count: usize,
        policy_generation: u64,
    ) -> Result<(), AgentCallDecodeError> {
        let count = archive_count(first_sequence, through_sequence, count)?;
        if generation == 0 || policy_generation == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_PREPARE_DURABLE_ARCHIVE)?;
        frame.r10 = generation;
        frame.r11 = first_sequence;
        frame.r12 = through_sequence;
        frame.r13 = count;
        frame.r14 = DURABLE_ARCHIVE_MANIFEST_BYTES as u64;
        frame.r15 = DURABLE_ARCHIVE_REQUEST_BYTES as u64;
        frame.rbp = policy_generation;
        Ok(())
    }

    pub fn encode_durable_archive_commit_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        first_sequence: u64,
        through_sequence: u64,
        count: usize,
        digest: EventArchiveDigest,
    ) -> Result<(), AgentCallDecodeError> {
        let count = archive_count(first_sequence, through_sequence, count)?;
        self.encode_reply(frame, nonce, AGENT_CALL_COMMIT_DURABLE_ARCHIVE)?;
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

    pub fn encode_durable_archive_sign_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        generation: u64,
        policy_generation: u64,
        signature_algorithm: u16,
    ) -> Result<(), AgentCallDecodeError> {
        if generation == 0 || policy_generation == 0 || signature_algorithm == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_SIGN_DURABLE_ARCHIVE)?;
        frame.r10 = generation;
        frame.r11 = policy_generation;
        frame.r12 = u64::from(signature_algorithm);
        Ok(())
    }
}

fn archive_count(
    first_sequence: u64,
    through_sequence: u64,
    count: usize,
) -> Result<u64, AgentCallDecodeError> {
    let count = u64::try_from(count).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
    if first_sequence == 0
        || through_sequence < first_sequence
        || count == 0
        || through_sequence - first_sequence + 1 != count
    {
        Err(AgentCallDecodeError::InvalidPayload)
    } else {
        Ok(count)
    }
}
