//! Owned type-state sequence for native Verifier Agent calls.
//!
//! This x86 CPU-layer module preserves one Verifier frame across result
//! inspection, target verification, and own-task completion. Semantic syscalls
//! remain in the Verifier task adapter; replies require acknowledged states.

mod completed;

use agent_kernel_core::{TaskId, TaskResult};
use agent_kernel_x86_64::{
    agent_call::{AgentCallContext, AgentCallRequest},
    context::SavedAgentFrame,
};

use super::{call, runtime::AgentCpuRuntime, storage, PreemptedAgentCpu};
use crate::agent_memory::PreparedAgentMemory;

pub(crate) use completed::CompletedVerifierCpu;

struct VerifierCallSession {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    frame: SavedAgentFrame,
    context: AgentCallContext,
    target: TaskId,
    result: Option<TaskResult>,
    describe_return_offset: u32,
    inspection_return_offset: u32,
    verification_return_offset: u32,
    nonce: u64,
}

pub(crate) struct RequestedTaskInspectionCpu(VerifierCallSession);
pub(crate) struct AcknowledgedTaskInspectionCpu(VerifierCallSession);
pub(crate) struct RequestedTaskVerificationCpu(VerifierCallSession);
pub(crate) struct AcknowledgedTaskVerificationCpu(VerifierCallSession);

impl PreemptedAgentCpu {
    pub(crate) fn resume_until_task_inspection(
        mut self,
        target: TaskId,
    ) -> Option<RequestedTaskInspectionCpu> {
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
            _ => return None,
        };
        let describe_return_offset = describe.return_offset();
        let mut reply = describe.into_frame();
        self.context
            .encode_describe_reply(reply.frame_mut(), nonce)
            .ok()?;

        storage::begin_dispatch(roots)?;
        call::resume_owned(&mut reply, roots, layout)?;
        let inspection = call::capture(self.runtime.kernel_stack, roots, layout)?;
        if !self
            .context
            .matches_task_result_inspection(inspection.request(), nonce, target)
        {
            return None;
        }
        let inspection_return_offset = inspection.return_offset();
        Some(RequestedTaskInspectionCpu(VerifierCallSession {
            memory: self.memory,
            runtime: self.runtime,
            frame: inspection.into_frame(),
            context: self.context,
            target,
            result: None,
            describe_return_offset,
            inspection_return_offset,
            verification_return_offset: 0,
            nonce,
        }))
    }
}

impl RequestedTaskInspectionCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        2
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        4
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.0.context
    }

    pub(crate) const fn target(&self) -> TaskId {
        self.0.target
    }

    pub(crate) const fn describe_return_offset(&self) -> u32 {
        self.0.describe_return_offset
    }

    pub(crate) const fn inspection_return_offset(&self) -> u32 {
        self.0.inspection_return_offset
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.0.nonce
    }

    pub(crate) fn acknowledge(
        mut self,
        result: TaskResult,
    ) -> Option<AcknowledgedTaskInspectionCpu> {
        self.0
            .context
            .encode_task_result_inspection_reply(self.0.frame.frame_mut(), self.0.nonce, result)
            .ok()?;
        self.0.result = Some(result);
        Some(AcknowledgedTaskInspectionCpu(self.0))
    }
}

impl AcknowledgedTaskInspectionCpu {
    pub(crate) fn resume_until_task_verification(mut self) -> Option<RequestedTaskVerificationCpu> {
        let roots = self.0.memory.roots();
        let layout = self.0.memory.layout();
        storage::begin_dispatch(roots)?;
        call::resume_owned(&mut self.0.frame, roots, layout)?;
        let verification = call::capture(self.0.runtime.kernel_stack, roots, layout)?;
        if !self.0.context.matches_task_verification(
            verification.request(),
            self.0.nonce,
            self.0.target,
        ) {
            return None;
        }
        self.0.verification_return_offset = verification.return_offset();
        self.0.frame = verification.into_frame();
        Some(RequestedTaskVerificationCpu(self.0))
    }
}

impl RequestedTaskVerificationCpu {
    pub(crate) const fn call_count(&self) -> u8 {
        3
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        6
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.0.context
    }

    pub(crate) const fn target(&self) -> TaskId {
        self.0.target
    }

    pub(crate) fn result(&self) -> Option<TaskResult> {
        self.0.result
    }

    pub(crate) const fn verification_return_offset(&self) -> u32 {
        self.0.verification_return_offset
    }

    pub(crate) fn acknowledge(mut self) -> Option<AcknowledgedTaskVerificationCpu> {
        self.0
            .context
            .encode_task_verification_reply(self.0.frame.frame_mut(), self.0.nonce)
            .ok()?;
        Some(AcknowledgedTaskVerificationCpu(self.0))
    }
}

impl AcknowledgedTaskVerificationCpu {
    pub(crate) fn resume_until_completion(mut self) -> Option<CompletedVerifierCpu> {
        let roots = self.0.memory.roots();
        let layout = self.0.memory.layout();
        storage::begin_dispatch(roots)?;
        call::resume_owned(&mut self.0.frame, roots, layout)?;
        let completion = call::capture(self.0.runtime.kernel_stack, roots, layout)?;
        if !self
            .0
            .context
            .matches_completion(completion.request(), self.0.nonce)
        {
            return None;
        }
        Some(CompletedVerifierCpu {
            describe_return_offset: self.0.describe_return_offset,
            inspection_return_offset: self.0.inspection_return_offset,
            verification_return_offset: self.0.verification_return_offset,
            completion_return_offset: completion.return_offset(),
            nonce: self.0.nonce,
            context: self.0.context,
            target: self.0.target,
            result: self.0.result?,
        })
    }
}
