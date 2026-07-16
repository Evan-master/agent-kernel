//! Two-Worker semantic schedule for physical x86 context rotation.
//!
//! This boot adapter owns only type-state ordering between architecture evidence
//! and public task syscalls. Setup admits both Workers; transition helpers prove
//! FIFO queue state across two expiries, returning result calls, and terminal
//! completion calls.

mod completed;
mod result_transition;
mod setup;
mod transitions;

use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageRecord, CapabilityId, TaskId, TaskResult,
};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::{
    agent_cpu::{
        AcknowledgedTaskResultCpu, CompletedAgentCpu, PreemptedAgentCpu, RequestedTaskResultCpu,
    },
    X86BootedKernel,
};

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

pub(super) struct SecondRunningFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct FirstResumedFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct SecondResumedFlow {
    first: WorkerTask,
    second: WorkerTask,
}

pub(super) struct FirstResultSubmittedFlow {
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

    pub(super) fn expire_first_and_dispatch_second(
        self,
        booted: &mut X86BootedKernel,
        cpu: &PreemptedAgentCpu,
    ) -> Option<SecondRunningFlow> {
        transitions::expire_and_dispatch(booted, self.first, self.second, cpu, 0)?;
        Some(SecondRunningFlow {
            first: self.first,
            second: self.second,
        })
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

    pub(super) fn dispatch_first(self, booted: &mut X86BootedKernel) -> Option<TimerTaskFlow> {
        setup::dispatch_first(booted, self.first, self.second)?;
        Some(TimerTaskFlow {
            first: self.first,
            second: self.second,
        })
    }
}

impl SecondRunningFlow {
    pub(super) fn expire_second_and_dispatch_first(
        self,
        booted: &mut X86BootedKernel,
        cpu: &PreemptedAgentCpu,
    ) -> Option<FirstResumedFlow> {
        transitions::expire_and_dispatch(booted, self.second, self.first, cpu, 1)?;
        Some(FirstResumedFlow {
            first: self.first,
            second: self.second,
        })
    }
}

impl FirstResumedFlow {
    pub(super) fn submit_first_result(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedTaskResultCpu,
    ) -> Option<(FirstResultSubmittedFlow, AcknowledgedTaskResultCpu)> {
        let acknowledged =
            result_transition::submit(booted, self.first, Some(self.second), None, cpu)?;
        Some((
            FirstResultSubmittedFlow {
                first: self.first,
                second: self.second,
            },
            acknowledged,
        ))
    }
}

impl FirstResultSubmittedFlow {
    pub(super) fn complete_first_and_dispatch_second(
        self,
        booted: &mut X86BootedKernel,
        cpu: CompletedAgentCpu,
    ) -> Option<SecondResumedFlow> {
        transitions::complete_and_dispatch(booted, self.first, self.second, cpu, 1)?;
        Some(SecondResumedFlow {
            first: self.first,
            second: self.second,
        })
    }
}

impl SecondResumedFlow {
    pub(super) fn submit_second_result(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedTaskResultCpu,
    ) -> Option<(SecondResultSubmittedFlow, AcknowledgedTaskResultCpu)> {
        let acknowledged =
            result_transition::submit(booted, self.second, None, Some(self.first), cpu)?;
        Some((
            SecondResultSubmittedFlow {
                first: self.first,
                second: self.second,
            },
            acknowledged,
        ))
    }
}

impl SecondResultSubmittedFlow {
    pub(super) fn record_second_completion(
        self,
        booted: &mut X86BootedKernel,
        cpu: CompletedAgentCpu,
    ) -> Option<CompletedWorkerTasks> {
        transitions::record_final_completion(booted, self.second, self.first, cpu)
            .then_some(CompletedWorkerTasks::new(self.first, self.second))
    }
}
