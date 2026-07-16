//! Two-Worker semantic schedule for physical x86 context rotation.
//!
//! This boot adapter owns only type-state ordering between architecture evidence
//! and public task syscalls. Setup admits both Workers; transition helpers prove
//! FIFO queue state across two expiries, returning result calls, and terminal
//! completion calls.

mod completed;
mod mailbox;
mod message_transition;
mod result_transition;
mod setup;
mod transitions;
mod wait_transition;

use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageRecord, CapabilityId, TaskId, TaskResult,
    WaiterId,
};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::{agent_cpu::PreemptedAgentCpu, X86BootedKernel};

pub(super) use completed::{CompletedWorkerTasks, VerificationSubject};

pub(super) const WORKER_A: AgentId = AgentId::new(3);
pub(super) const WORKER_B: AgentId = AgentId::new(4);
pub(super) const TASK_QUANTUM: u64 = 1;

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

pub(super) struct TimerTaskFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct FirstRunningFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct SecondResumedFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct SecondWaitingFlow {
    first: WorkerTask,
    second: WorkerTask,
    waiter: WaiterId,
}

pub(super) struct FirstResumedFlow {
    first: WorkerTask,
    second: WorkerTask,
    waiter: WaiterId,
}

pub(super) struct FirstResultSubmittedFlow {
    first: WorkerTask,
    second: WorkerTask,
    waiter: WaiterId,
}

pub(super) struct FirstMessageSentFlow {
    first: WorkerTask,
    second: WorkerTask,
    waiter: WaiterId,
}

pub(super) struct SecondRedispatchedFlow {
    first: WorkerTask,
    second: WorkerTask,
    waiter: WaiterId,
}

pub(super) struct SecondMessageReceivedFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct SecondMessageAcknowledgedFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct SecondResultSubmittedFlow {
    first: WorkerTask,
    second: WorkerTask,
}

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

    pub(super) fn expire_second_and_dispatch_first(
        self,
        booted: &mut X86BootedKernel,
        cpu: &PreemptedAgentCpu,
    ) -> Option<(FirstRunningFlow, agent_kernel_core::RunQueueEntry)> {
        let dispatched = transitions::expire_and_dispatch(booted, self.second, self.first, cpu, 0)?;
        Some((
            FirstRunningFlow {
                first: self.first,
                second: self.second,
            },
            dispatched,
        ))
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

    pub(super) fn dispatch_second(
        self,
        booted: &mut X86BootedKernel,
    ) -> Option<(TimerTaskFlow, agent_kernel_core::RunQueueEntry)> {
        let dispatched = setup::dispatch_second(booted, self.first, self.second)?;
        Some((
            TimerTaskFlow {
                first: self.first,
                second: self.second,
            },
            dispatched,
        ))
    }
}

impl FirstRunningFlow {
    pub(super) fn expire_first_and_dispatch_second(
        self,
        booted: &mut X86BootedKernel,
        cpu: &PreemptedAgentCpu,
    ) -> Option<SecondResumedFlow> {
        transitions::expire_and_dispatch(booted, self.first, self.second, cpu, 1)?;
        Some(SecondResumedFlow {
            first: self.first,
            second: self.second,
        })
    }
}
