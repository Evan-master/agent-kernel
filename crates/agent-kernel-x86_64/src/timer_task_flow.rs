//! Agent Task admission and scheduler bottom half for one physical timer tick.
//!
//! This x86 boot-layer module prepares a capability-scoped Worker task through
//! public kernel syscalls, dispatches it with quantum one, and applies only a
//! validated architecture timer signal. It never edits core stores directly.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, EventKind,
    IntentKind, RunQueueEntry, TaskId, TaskStatus, VerificationRequirement,
};

use crate::{pit_timer::PitTimerSignal, X86BootedKernel};

const WORKER: AgentId = AgentId::new(3);
const TASK_QUANTUM: u64 = 1;

pub struct TimerTaskFlow {
    task: TaskId,
}

impl TimerTaskFlow {
    pub fn prepare(booted: &mut X86BootedKernel) -> Option<Self> {
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

    pub fn apply_tick(self, booted: &mut X86BootedKernel, signal: PitTimerSignal) -> bool {
        if signal.count() != 1 {
            return false;
        }

        let kernel = booted.kernel_mut();
        let Ok(event) = kernel.sys_tick_task(WORKER, self.task) else {
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

        event.kind == EventKind::TaskQuantumExpired
            && event.task == Some(self.task)
            && event.task_ticks == Some(1)
            && event.task_quantum == Some(0)
            && task.status == TaskStatus::Accepted
            && task.run_ticks == 1
            && task.quantum_remaining == 0
            && context.state == AgentExecutionState::Idle
            && context.task.is_none()
            && kernel.run_queue()
                == [RunQueueEntry {
                    task: self.task,
                    agent: WORKER,
                }]
    }
}
