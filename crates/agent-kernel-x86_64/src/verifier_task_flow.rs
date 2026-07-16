//! Admission, authority, and terminal evidence for the native Verifier task.
//!
//! The Verifier owns task-scoped execution authority and a separate attenuated
//! Verify capability. Physical call sequencing belongs to the generic runtime
//! executor; this module keeps only trusted semantic metadata.

mod runtime;
mod setup;

use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageRecord, CapabilityId, RunQueueEntry, TaskId,
};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::{
    native_agent_executor::NativeVerifyAuthority,
    timer_task_flow::{CompletedWorkerTasks, VerificationSubject},
    X86BootedKernel,
};

pub(super) const VERIFIER: AgentId = AgentId::new(5);

#[derive(Copy, Clone)]
struct VerifierTask {
    agent: AgentId,
    task: TaskId,
    image: AgentImageId,
    task_capability: CapabilityId,
    verify_capability: CapabilityId,
    subject: VerificationSubject,
}

impl VerifierTask {
    const fn call_context(self) -> Option<AgentCallContext> {
        AgentCallContext::new(self.agent, self.task, self.image, self.task_capability)
    }
}

pub(super) struct VerifierTaskFlow;

pub(super) struct PreparedVerifierFlow {
    verifier: VerifierTask,
}

pub(super) struct CompletedVerifierFlow;

impl VerifierTaskFlow {
    pub(super) fn prepare(
        booted: &mut X86BootedKernel,
        subject: VerificationSubject,
        digest: AgentImageDigest,
    ) -> Option<PreparedVerifierFlow> {
        Some(PreparedVerifierFlow {
            verifier: setup::prepare(booted, subject, digest)?,
        })
    }
}

impl PreparedVerifierFlow {
    pub(super) fn call_context(&self) -> Option<AgentCallContext> {
        self.verifier.call_context()
    }

    pub(super) fn image_record(&self, booted: &X86BootedKernel) -> Option<AgentImageRecord> {
        booted.kernel().agent_image(self.verifier.image).ok()
    }

    pub(super) fn runtime_authority(&self) -> Option<NativeVerifyAuthority> {
        NativeVerifyAuthority::new(self.verifier.agent, self.verifier.verify_capability)
    }

    pub(super) fn queue_after_workers_for_runtime(
        &self,
        booted: &mut X86BootedKernel,
        workers: &CompletedWorkerTasks,
        predecessor: Option<RunQueueEntry>,
    ) -> Option<()> {
        runtime::queue(booted, self.verifier, workers, predecessor)
    }

    pub(super) fn completed_after_runtime(
        self,
        booted: &X86BootedKernel,
        workers: &CompletedWorkerTasks,
    ) -> Option<CompletedVerifierFlow> {
        runtime::completed(booted, self.verifier, workers).then_some(CompletedVerifierFlow)
    }
}
