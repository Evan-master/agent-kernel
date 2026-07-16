//! Scheduled native Verifier task bound to physical x86 call evidence.
//!
//! The Verifier owns task-scoped execution authority and a separate attenuated
//! resource-scoped Verify capability. Type states prevent inspection,
//! verification, or completion before the matching ring-3 request exists.

mod setup;
mod transitions;

use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageRecord, CapabilityId, RunQueueEntry, TaskId,
    TaskResult,
};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::{
    agent_cpu::{
        AcknowledgedTaskInspectionCpu, AcknowledgedTaskVerificationCpu, CompletedVerifierCpu,
        PreemptedAgentCpu, RequestedTaskInspectionCpu, RequestedTaskVerificationCpu,
    },
    timer_task_flow::{CompletedWorkerTasks, VerificationSubject},
    X86BootedKernel,
};

pub(super) const VERIFIER: AgentId = AgentId::new(5);
pub(super) const VERIFIER_QUANTUM: u64 = 1;

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

pub(super) struct RunningVerifierFlow {
    verifier: VerifierTask,
    workers: CompletedWorkerTasks,
}

pub(super) struct ResumedVerifierFlow {
    verifier: VerifierTask,
    workers: CompletedWorkerTasks,
}

pub(super) struct InspectedVerifierFlow {
    verifier: VerifierTask,
    workers: CompletedWorkerTasks,
}

pub(super) struct VerifiedSubjectFlow {
    verifier: VerifierTask,
    workers: CompletedWorkerTasks,
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

    pub(super) fn dispatch_after_workers(
        self,
        booted: &mut X86BootedKernel,
        workers: CompletedWorkerTasks,
    ) -> Option<(RunningVerifierFlow, RunQueueEntry)> {
        let dispatched = transitions::dispatch(booted, self.verifier, &workers)?;
        Some((
            RunningVerifierFlow {
                verifier: self.verifier,
                workers,
            },
            dispatched,
        ))
    }
}

impl RunningVerifierFlow {
    pub(super) fn expire_and_redispatch(
        self,
        booted: &mut X86BootedKernel,
        cpu: &PreemptedAgentCpu,
    ) -> Option<ResumedVerifierFlow> {
        transitions::expire_and_redispatch(booted, self.verifier, &self.workers, cpu)?;
        Some(ResumedVerifierFlow {
            verifier: self.verifier,
            workers: self.workers,
        })
    }
}

impl ResumedVerifierFlow {
    pub(super) fn inspect_subject(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedTaskInspectionCpu,
    ) -> Option<(InspectedVerifierFlow, AcknowledgedTaskInspectionCpu)> {
        let acknowledged = transitions::inspect(booted, self.verifier, &self.workers, cpu)?;
        Some((
            InspectedVerifierFlow {
                verifier: self.verifier,
                workers: self.workers,
            },
            acknowledged,
        ))
    }
}

impl InspectedVerifierFlow {
    pub(super) fn verify_subject(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedTaskVerificationCpu,
    ) -> Option<(VerifiedSubjectFlow, AcknowledgedTaskVerificationCpu)> {
        let acknowledged = transitions::verify(booted, self.verifier, &self.workers, cpu)?;
        Some((
            VerifiedSubjectFlow {
                verifier: self.verifier,
                workers: self.workers,
            },
            acknowledged,
        ))
    }
}

impl VerifiedSubjectFlow {
    pub(super) fn complete(
        self,
        booted: &mut X86BootedKernel,
        cpu: CompletedVerifierCpu,
    ) -> Option<CompletedVerifierFlow> {
        transitions::complete(booted, self.verifier, &self.workers, cpu)?;
        Some(CompletedVerifierFlow)
    }
}

const fn subject_result(verifier: VerifierTask) -> TaskResult {
    verifier.subject.result()
}
