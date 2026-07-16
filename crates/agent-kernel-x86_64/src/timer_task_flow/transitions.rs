//! Scheduler-state validation for physical preemption and completion evidence.
//!
//! Each helper performs one public lifecycle transition, checks its event, and
//! proves the FIFO queue before allowing the next physical Agent dispatch.

use agent_kernel_core::{AgentExecutionState, EventKind, RunQueueEntry, TaskStatus};

use super::{completed::task_valid as completed_task_valid, WorkerTask, TASK_QUANTUM};
use crate::{
    agent_cpu::{CompletedMailboxReceiverCpu, CompletedMailboxSenderCpu, PreemptedAgentCpu},
    X86BootedKernel,
};

pub(super) trait CompletionEvidence {
    const EXPECTED_CALLS: u8;

    fn call_count(&self) -> u8;
    fn address_space_switch_count(&self) -> u8;
    fn context(&self) -> agent_kernel_x86_64::agent_call::AgentCallContext;
}

pub(super) fn expire_and_dispatch(
    booted: &mut X86BootedKernel,
    running: WorkerTask,
    next: WorkerTask,
    cpu: &PreemptedAgentCpu,
    next_prior_ticks: u64,
) -> Option<RunQueueEntry> {
    if cpu.tick_count() != 1 {
        return None;
    }
    let expiry = booted
        .kernel_mut()
        .sys_tick_task(running.agent, running.task)
        .ok()?;
    if expiry.kind != EventKind::TaskQuantumExpired
        || expiry.task != Some(running.task)
        || expiry.task_ticks != Some(1)
        || expiry.task_quantum != Some(0)
        || !idle_task_valid(booted, running, 1)
        || !idle_task_valid(booted, next, next_prior_ticks)
        || booted.kernel().run_queue()
            != [
                RunQueueEntry {
                    task: next.task,
                    agent: next.agent,
                },
                RunQueueEntry {
                    task: running.task,
                    agent: running.agent,
                },
            ]
    {
        return None;
    }
    let dispatched = booted
        .kernel_mut()
        .sys_dispatch_next_ready_with_quantum(TASK_QUANTUM)
        .ok()?;
    if dispatched
        != (RunQueueEntry {
            task: next.task,
            agent: next.agent,
        })
    {
        return None;
    }
    running_and_queue_valid(booted, next, running, next_prior_ticks, 1)?;
    Some(dispatched)
}

pub(super) fn complete_and_dispatch<E: CompletionEvidence>(
    booted: &mut X86BootedKernel,
    running: WorkerTask,
    next: WorkerTask,
    cpu: E,
    next_prior_ticks: u64,
) -> Option<()> {
    if !completion_evidence_valid(&cpu, running) {
        return None;
    }
    let completed = booted
        .kernel_mut()
        .sys_complete_task(running.agent, running.capability, running.task)
        .ok()?;
    if completed.kind != EventKind::TaskCompleted
        || completed.agent != running.agent
        || completed.task != Some(running.task)
        || completed.capability != Some(running.capability)
        || !completed_task_valid(booted, running, 1)
        || !idle_task_valid(booted, next, next_prior_ticks)
        || booted.kernel().run_queue()
            != [RunQueueEntry {
                task: next.task,
                agent: next.agent,
            }]
    {
        return None;
    }
    let dispatched = booted
        .kernel_mut()
        .sys_dispatch_next_ready_with_quantum(TASK_QUANTUM)
        .ok()?;
    if dispatched
        != (RunQueueEntry {
            task: next.task,
            agent: next.agent,
        })
    {
        return None;
    }
    running_after_completion_valid(booted, next, running, next_prior_ticks)
}

pub(super) fn record_final_completion<E: CompletionEvidence>(
    booted: &mut X86BootedKernel,
    running: WorkerTask,
    completed: WorkerTask,
    cpu: E,
) -> bool {
    if !completion_evidence_valid(&cpu, running) {
        return false;
    }
    let kernel = booted.kernel_mut();
    let Ok(event) = kernel.sys_complete_task(running.agent, running.capability, running.task)
    else {
        return false;
    };
    event.kind == EventKind::TaskCompleted
        && event.agent == running.agent
        && event.task == Some(running.task)
        && event.capability == Some(running.capability)
        && kernel.run_queue().is_empty()
        && completed_task_valid(booted, running, 1)
        && completed_task_valid(booted, completed, 1)
}

fn completion_evidence_valid<E: CompletionEvidence>(cpu: &E, running: WorkerTask) -> bool {
    cpu.call_count() == E::EXPECTED_CALLS
        && cpu.address_space_switch_count() == E::EXPECTED_CALLS * 2
        && running.call_context() == Some(cpu.context())
}

macro_rules! impl_completion_evidence {
    ($completed:ty, $calls:expr) => {
        impl CompletionEvidence for $completed {
            const EXPECTED_CALLS: u8 = $calls;

            fn call_count(&self) -> u8 {
                self.call_count()
            }

            fn address_space_switch_count(&self) -> u8 {
                self.address_space_switch_count()
            }

            fn context(&self) -> agent_kernel_x86_64::agent_call::AgentCallContext {
                self.context()
            }
        }
    };
}

impl_completion_evidence!(CompletedMailboxSenderCpu, 4);
impl_completion_evidence!(CompletedMailboxReceiverCpu, 5);

fn running_after_completion_valid(
    booted: &X86BootedKernel,
    running: WorkerTask,
    completed: WorkerTask,
    running_prior_ticks: u64,
) -> Option<()> {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == running.task)?;
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == running.agent)?;
    let event = kernel.events().last()?;
    (task.status == TaskStatus::Running
        && task.assignee == Some(running.agent)
        && task.delegated_capability == Some(running.capability)
        && task.result.is_none()
        && task.run_ticks == running_prior_ticks
        && task.quantum_remaining == TASK_QUANTUM
        && context.state == AgentExecutionState::Running
        && context.task == Some(running.task)
        && context.run_ticks == running_prior_ticks
        && context.quantum_remaining == TASK_QUANTUM
        && event.kind == EventKind::TaskDispatched
        && event.task == Some(running.task)
        && kernel.run_queue().is_empty()
        && completed_task_valid(booted, completed, 1))
    .then_some(())
}

fn running_and_queue_valid(
    booted: &X86BootedKernel,
    running: WorkerTask,
    queued: WorkerTask,
    running_prior_ticks: u64,
    queued_ticks: u64,
) -> Option<()> {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == running.task)?;
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == running.agent)?;
    let event = kernel.events().last()?;
    (task.status == TaskStatus::Running
        && task.assignee == Some(running.agent)
        && task.delegated_capability == Some(running.capability)
        && task.result.is_none()
        && task.run_ticks == running_prior_ticks
        && task.quantum_remaining == TASK_QUANTUM
        && context.state == AgentExecutionState::Running
        && context.task == Some(running.task)
        && context.run_ticks == running_prior_ticks
        && context.quantum_remaining == TASK_QUANTUM
        && event.kind == EventKind::TaskDispatched
        && event.task == Some(running.task)
        && kernel.run_queue()
            == [RunQueueEntry {
                task: queued.task,
                agent: queued.agent,
            }]
        && idle_task_valid(booted, queued, queued_ticks))
    .then_some(())
}

fn idle_task_valid(booted: &X86BootedKernel, worker: WorkerTask, ticks: u64) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == worker.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == worker.agent);
    matches!(task, Some(task) if task.status == TaskStatus::Accepted
        && task.assignee == Some(worker.agent)
        && task.delegated_capability == Some(worker.capability)
        && task.result.is_none()
        && task.run_ticks == ticks)
        && matches!(context, Some(context) if context.state == AgentExecutionState::Idle && context.task.is_none())
}
