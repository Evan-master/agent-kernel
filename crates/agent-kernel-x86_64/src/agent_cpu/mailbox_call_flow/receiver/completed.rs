//! Terminal evidence for the five-call native mailbox receiver sequence.
//!
//! This CPU-layer child owns the final trusted context, message, result, and
//! return offsets consumed by the semantic completion adapter.

use agent_kernel_core::{MessageRecord, TaskResult};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use super::AcknowledgedReceiverResultCpu;

pub(crate) struct CompletedMailboxReceiverCpu {
    context: AgentCallContext,
    nonce: u64,
    message: MessageRecord,
    result: TaskResult,
    return_offsets: [u32; 5],
}

impl AcknowledgedReceiverResultCpu {
    pub(crate) fn resume_until_completion(self) -> Option<CompletedMailboxReceiverCpu> {
        let (session, request, completion_return_offset) =
            self.0.acknowledged.received.request.session.resume_next()?;
        if !session
            .context()
            .matches_completion(request, session.nonce())
        {
            return None;
        }
        Some(CompletedMailboxReceiverCpu {
            context: session.context(),
            nonce: session.nonce(),
            message: self.0.acknowledged.received.message,
            result: self.0.result,
            return_offsets: [
                session.describe_return_offset(),
                self.0.acknowledged.received.request.receive_return_offset,
                self.0.acknowledged.acknowledgement_return_offset,
                self.0.result_return_offset,
                completion_return_offset,
            ],
        })
    }
}

impl CompletedMailboxReceiverCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        5
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        10
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.context
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.nonce
    }

    pub(crate) const fn message(&self) -> MessageRecord {
        self.message
    }

    pub(crate) const fn result(&self) -> TaskResult {
        self.result
    }

    pub(crate) const fn return_offsets(&self) -> [u32; 5] {
        self.return_offsets
    }
}
