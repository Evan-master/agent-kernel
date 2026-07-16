//! Admission and terminal evidence for the two native Worker tasks.
//!
//! Setup owns deterministic Agent/task/image creation. Physical scheduling and
//! Agent Call routing belong to the generic runtime executor; this module keeps
//! only trusted Worker metadata and final semantic predicates.

mod completed;
mod setup;

use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageRecord, CapabilityId, TaskId, TaskResult,
};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::X86BootedKernel;

pub(super) use completed::{CompletedWorkerTasks, VerificationSubject};

pub(super) const WORKER_A: AgentId = AgentId::new(3);
pub(super) const WORKER_B: AgentId = AgentId::new(4);

#[derive(Copy, Clone)]
pub(super) struct WorkerTask {
    agent: AgentId,
    task: TaskId,
    image: AgentImageId,
    capability: CapabilityId,
    result: TaskResult,
}

impl WorkerTask {
    const fn new(
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        capability: CapabilityId,
        result: TaskResult,
    ) -> Self {
        Self {
            agent,
            task,
            image,
            capability,
            result,
        }
    }

    const fn call_context(self) -> Option<AgentCallContext> {
        AgentCallContext::new(self.agent, self.task, self.image, self.capability)
    }
}

pub(super) struct QueuedTimerTaskFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct TimerTaskFlow;

impl TimerTaskFlow {
    pub(super) fn prepare(
        booted: &mut X86BootedKernel,
        first_digest: AgentImageDigest,
        second_digest: AgentImageDigest,
        first_result: TaskResult,
        second_result: TaskResult,
    ) -> Option<QueuedTimerTaskFlow> {
        let (first, second) = setup::prepare(
            booted,
            first_digest,
            second_digest,
            first_result,
            second_result,
        )?;
        Some(QueuedTimerTaskFlow { first, second })
    }
}

impl QueuedTimerTaskFlow {
    pub(super) const fn verification_subject(&self) -> VerificationSubject {
        VerificationSubject::new(self.first.task, self.first.result)
    }

    pub(super) fn call_contexts(&self) -> Option<(AgentCallContext, AgentCallContext)> {
        Some((self.first.call_context()?, self.second.call_context()?))
    }

    pub(super) fn image_records(
        &self,
        booted: &X86BootedKernel,
    ) -> Option<(AgentImageRecord, AgentImageRecord)> {
        let first = booted.kernel().agent_image(self.first.image).ok()?;
        let second = booted.kernel().agent_image(self.second.image).ok()?;
        (first.id != second.id && first.digest != second.digest).then_some((first, second))
    }

    pub(super) fn completed_after_runtime(
        self,
        booted: &X86BootedKernel,
    ) -> Option<CompletedWorkerTasks> {
        let completed = CompletedWorkerTasks::new(self.first, self.second);
        completed.both_completed(booted).then_some(completed)
    }
}
