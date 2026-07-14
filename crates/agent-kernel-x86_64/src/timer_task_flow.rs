//! Agent Task lifecycle adapter for physical CPU preemption and resume.
//!
//! This x86 boot-layer module prepares a capability-scoped Worker through
//! public syscalls, then uses unforgeable architecture type-state evidence to
//! order quantum expiry, redispatch, and cooperative yield. It never edits core
//! stores directly.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, EventKind,
    IntentKind, RunQueueEntry, TaskId, TaskStatus, VerificationRequirement,
};

use crate::{
    agent_cpu::{PreemptedAgentCpu, YieldedAgentCpu},
    X86BootedKernel,
};

const WORKER: AgentId = AgentId::new(3);
const TASK_QUANTUM: u64 = 1;

pub(super) struct TimerTaskFlow {
    task: TaskId,
}

pub(super) struct QueuedTimerTaskFlow {
    task: TaskId,
}

pub(super) struct RunningTimerTaskFlow {
    task: TaskId,
}

impl TimerTaskFlow {
    pub(super) fn prepare(booted: &mut X86BootedKernel) -> Option<Self> {
        let report = *booted.report();
        let kernel = booted.kernel_mut();
        kernel.sys_register_agent(WORKER).ok()?;
        let intent = kernel
            .sys_declare_intent(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                IntentKind::Act,
                VerificationRequirement::Required,
            )
            .ok()?;
        let task = kernel
            .sys_create_task(report.bootstrap_agent, report.bootstrap_capability, intent)
            .ok()?;
        kernel
            .sys_delegate_task(
                report.bootstrap_agent,
                report.bootstrap_capability,
                task,
                WORKER,
            )
            .ok()?;
        let worker_capability = kernel
            .tasks()
            .iter()
            .find(|record| record.id == task)?
            .delegated_capability?;
        let image = kernel
            .sys_register_agent_image(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                AgentImageKind::Worker,
                AgentImageDigest::new([0x57; 32]),
                1,
                1,
            )
            .ok()?;
        kernel
            .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
            .ok()?;
        kernel
            .sys_launch_task_agent(
                WORKER,
                worker_capability,
                task,
                image,
                AgentEntryKind::Worker,
            )
            .ok()?;
        kernel.sys_accept_task(WORKER, task).ok()?;
        kernel.sys_enqueue_task(WORKER, task).ok()?;
        if kernel
            .sys_dispatch_next_with_quantum(WORKER, TASK_QUANTUM)
            .ok()?
            != task
        {
            return None;
        }

        Some(Self { task })
    }

    pub(super) fn apply_preemption(
        self,
        booted: &mut X86BootedKernel,
        cpu: &PreemptedAgentCpu,
    ) -> Option<QueuedTimerTaskFlow> {
        if cpu.tick_count() != 1 {
            return None;
        }

        let kernel = booted.kernel_mut();
        let event = kernel.sys_tick_task(WORKER, self.task).ok()?;
        let task = kernel.tasks().iter().find(|task| task.id == self.task)?;
        let context = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == WORKER)?;
        if event.kind != EventKind::TaskQuantumExpired
            || event.task != Some(self.task)
            || event.task_ticks != Some(1)
            || event.task_quantum != Some(0)
            || task.status != TaskStatus::Accepted
            || task.run_ticks != 1
            || task.quantum_remaining != 0
            || context.state != AgentExecutionState::Idle
            || context.task.is_some()
            || kernel.run_queue()
                != [RunQueueEntry {
                    task: self.task,
                    agent: WORKER,
                }]
        {
            return None;
        }

        Some(QueuedTimerTaskFlow { task: self.task })
    }
}

impl QueuedTimerTaskFlow {
    pub(super) fn redispatch(self, booted: &mut X86BootedKernel) -> Option<RunningTimerTaskFlow> {
        let kernel = booted.kernel_mut();
        if kernel
            .sys_dispatch_next_with_quantum(WORKER, TASK_QUANTUM)
            .ok()?
            != self.task
        {
            return None;
        }

        let task = kernel.tasks().iter().find(|task| task.id == self.task)?;
        let context = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == WORKER)?;
        let event = kernel.events().last()?;
        if event.kind != EventKind::TaskDispatched
            || event.task != Some(self.task)
            || event.task_quantum != Some(TASK_QUANTUM)
            || task.status != TaskStatus::Running
            || task.run_ticks != 1
            || task.quantum_remaining != TASK_QUANTUM
            || context.state != AgentExecutionState::Running
            || context.task != Some(self.task)
            || context.run_ticks != 1
            || context.quantum_remaining != TASK_QUANTUM
            || !kernel.run_queue().is_empty()
        {
            return None;
        }

        Some(RunningTimerTaskFlow { task: self.task })
    }
}

impl RunningTimerTaskFlow {
    pub(super) fn record_yield(self, booted: &mut X86BootedKernel, cpu: YieldedAgentCpu) -> bool {
        if cpu.yield_count() != 1 {
            return false;
        }

        let kernel = booted.kernel_mut();
        let Ok(event) = kernel.sys_yield_task(WORKER, self.task) else {
            return false;
        };
        let Some(task) = kernel.tasks().iter().find(|task| task.id == self.task) else {
            return false;
        };
        let Some(context) = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == WORKER)
        else {
            return false;
        };

        event.kind == EventKind::TaskYielded
            && event.task == Some(self.task)
            && task.status == TaskStatus::Accepted
            && task.run_ticks == 1
            && context.state == AgentExecutionState::Idle
            && context.task.is_none()
            && kernel.run_queue()
                == [RunQueueEntry {
                    task: self.task,
                    agent: WORKER,
                }]
    }
}
