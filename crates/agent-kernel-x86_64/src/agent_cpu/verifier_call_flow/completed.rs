//! Terminal physical evidence for one four-call Verifier execution.

use agent_kernel_core::{TaskId, TaskResult};
use agent_kernel_x86_64::agent_call::AgentCallContext;

pub(crate) struct CompletedVerifierCpu {
    pub(super) describe_return_offset: u32,
    pub(super) inspection_return_offset: u32,
    pub(super) verification_return_offset: u32,
    pub(super) completion_return_offset: u32,
    pub(super) nonce: u64,
    pub(super) context: AgentCallContext,
    pub(super) target: TaskId,
    pub(super) result: TaskResult,
}

impl CompletedVerifierCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        4
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        8
    }

    pub(crate) const fn describe_return_offset(&self) -> u32 {
        self.describe_return_offset
    }

    pub(crate) const fn inspection_return_offset(&self) -> u32 {
        self.inspection_return_offset
    }

    pub(crate) const fn verification_return_offset(&self) -> u32 {
        self.verification_return_offset
    }

    pub(crate) const fn completion_return_offset(&self) -> u32 {
        self.completion_return_offset
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.nonce
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.context
    }

    pub(crate) const fn target(&self) -> TaskId {
        self.target
    }

    pub(crate) const fn result(&self) -> TaskResult {
        self.result
    }
}
