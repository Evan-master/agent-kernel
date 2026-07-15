//! Owned type-state sequence for returning and terminal Agent calls.
//!
//! This CPU-layer module captures validated requests and preserves user frames.
//! Semantic task mutation remains in the timer/task adapter.

use agent_kernel_core::TaskResult;
use agent_kernel_x86_64::{
    agent_call::{AgentCallContext, AgentCallRequest},
    context::SavedAgentFrame,
};

use super::{call, runtime::AgentCpuRuntime, storage, PreemptedAgentCpu};
use crate::agent_memory::PreparedAgentMemory;

struct ResultCallSession {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    frame: SavedAgentFrame,
    context: AgentCallContext,
    result: TaskResult,
    describe_return_offset: u32,
    result_return_offset: u32,
    nonce: u64,
}

pub(crate) struct RequestedTaskResultCpu(ResultCallSession);

pub(crate) struct AcknowledgedTaskResultCpu(ResultCallSession);

pub(crate) struct CompletedAgentCpu {
    describe_return_offset: u32,
    result_return_offset: u32,
    completion_return_offset: u32,
    nonce: u64,
    context: AgentCallContext,
}

impl PreemptedAgentCpu {
    pub(crate) fn resume_until_task_result(mut self) -> Option<RequestedTaskResultCpu> {
        let roots = self.memory.roots();
        let layout = self.memory.layout();
        storage::begin_dispatch(roots)?;
        if !self.memory.release_for_agent_call() {
            return None;
        }
        call::resume_owned(&mut self.frame, roots, layout)?;

        let describe = call::capture(self.runtime.kernel_stack, roots, layout)?;
        let nonce = match describe.request() {
            AgentCallRequest::DescribeContext { nonce } => nonce,
            AgentCallRequest::Yield { .. }
            | AgentCallRequest::CompleteTask { .. }
            | AgentCallRequest::SubmitTaskResult { .. } => return None,
        };
        let describe_return_offset = describe.return_offset();
        let mut reply_frame = describe.into_frame();
        self.context
            .encode_describe_reply(reply_frame.frame_mut(), nonce)
            .ok()?;

        storage::begin_dispatch(roots)?;
        call::resume_owned(&mut reply_frame, roots, layout)?;
        let requested = call::capture(self.runtime.kernel_stack, roots, layout)?;
        let result = self.context.match_task_result(requested.request(), nonce)?;
        let result_return_offset = requested.return_offset();
        Some(RequestedTaskResultCpu(ResultCallSession {
            memory: self.memory,
            runtime: self.runtime,
            frame: requested.into_frame(),
            context: self.context,
            result,
            describe_return_offset,
            result_return_offset,
            nonce,
        }))
    }
}

impl RequestedTaskResultCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        2
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        4
    }

    pub(crate) const fn describe_return_offset(&self) -> u32 {
        self.0.describe_return_offset
    }

    pub(crate) const fn result_return_offset(&self) -> u32 {
        self.0.result_return_offset
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.0.nonce
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.0.context
    }

    pub(crate) const fn result(&self) -> TaskResult {
        self.0.result
    }

    pub(crate) fn acknowledge(mut self) -> Option<AcknowledgedTaskResultCpu> {
        self.0
            .context
            .encode_task_result_reply(self.0.frame.frame_mut(), self.0.nonce)
            .ok()?;
        Some(AcknowledgedTaskResultCpu(self.0))
    }
}

impl AcknowledgedTaskResultCpu {
    pub(crate) fn resume_until_completion(mut self) -> Option<CompletedAgentCpu> {
        let roots = self.0.memory.roots();
        let layout = self.0.memory.layout();
        storage::begin_dispatch(roots)?;
        call::resume_owned(&mut self.0.frame, roots, layout)?;
        let completed = call::capture(self.0.runtime.kernel_stack, roots, layout)?;
        if !self
            .0
            .context
            .matches_completion(completed.request(), self.0.nonce)
        {
            return None;
        }
        Some(CompletedAgentCpu {
            describe_return_offset: self.0.describe_return_offset,
            result_return_offset: self.0.result_return_offset,
            completion_return_offset: completed.return_offset(),
            nonce: self.0.nonce,
            context: self.0.context,
        })
    }
}

impl CompletedAgentCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        3
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        6
    }

    pub(crate) const fn describe_return_offset(&self) -> u32 {
        self.describe_return_offset
    }

    pub(crate) const fn result_return_offset(&self) -> u32 {
        self.result_return_offset
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
}
