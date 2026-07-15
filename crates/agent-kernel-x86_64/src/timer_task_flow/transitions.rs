//! Scheduler-state validation for physical preemption and yield evidence.
//!
//! Each helper performs one public lifecycle transition, checks its event, and
//! proves the FIFO queue before allowing the next physical Agent dispatch.

use agent_kernel_core::{AgentExecutionState, EventKind, RunQueueEntry, TaskStatus};

use super::{WorkerTask, TASK_QUANTUM};
use crate::{
    agent_cpu::{PreemptedAgentCpu, YieldedAgentCpu},
    X86BootedKernel,
};

pub(super) fn expire_and_dispatch(
    booted: &mut X86BootedKernel,
    running: WorkerTask,
    next: WorkerTask,
    cpu: &PreemptedAgentCpu,
    next_prior_ticks: u64,
) -> Option<()> {
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
    if booted
        .kernel_mut()
        .sys_dispatch_next_with_quantum(next.agent, TASK_QUANTUM)
        .ok()?
        != next.task
    {
        return None;
    }
    running_and_queue_valid(booted, next, running, next_prior_ticks, 1)
}

pub(super) fn yield_and_dispatch(
    booted: &mut X86BootedKernel,
    running: WorkerTask,
    next: WorkerTask,
    cpu: YieldedAgentCpu,
    next_prior_ticks: u64,
) -> Option<()> {
    if cpu.yield_count() != 1 || cpu.call_count() != 2 || cpu.address_space_switch_count() != 4 {
        return None;
    }
    let yielded = booted
        .kernel_mut()
        .sys_yield_task(running.agent, running.task)
        .ok()?;
    if yielded.kind != EventKind::TaskYielded
        || yielded.task != Some(running.task)
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
    if booted
        .kernel_mut()
        .sys_dispatch_next_with_quantum(next.agent, TASK_QUANTUM)
        .ok()?
        != next.task
    {
        return None;
    }
    running_and_queue_valid(booted, next, running, next_prior_ticks, 1)
}

pub(super) fn record_final_yield(
    booted: &mut X86BootedKernel,
    running: WorkerTask,
    queued: WorkerTask,
    cpu: YieldedAgentCpu,
) -> bool {
    if cpu.yield_count() != 1 || cpu.call_count() != 2 || cpu.address_space_switch_count() != 4 {
        return false;
    }
    let kernel = booted.kernel_mut();
    let Ok(event) = kernel.sys_yield_task(running.agent, running.task) else {
        return false;
    };
    event.kind == EventKind::TaskYielded
        && event.task == Some(running.task)
        && kernel.run_queue()
            == [
                RunQueueEntry {
                    task: queued.task,
                    agent: queued.agent,
                },
                RunQueueEntry {
                    task: running.task,
                    agent: running.agent,
                },
            ]
        && idle_task_valid(booted, running, 1)
        && idle_task_valid(booted, queued, 1)
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
    matches!(task, Some(task) if task.status == TaskStatus::Accepted && task.run_ticks == ticks)
        && matches!(context, Some(context) if context.state == AgentExecutionState::Idle && context.task.is_none())
}
