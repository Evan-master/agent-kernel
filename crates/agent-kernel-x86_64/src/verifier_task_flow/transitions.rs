//! Verifier scheduler and semantic transitions validated against CPU evidence.

mod result;

use agent_kernel_core::{AgentExecutionState, EventKind, RunQueueEntry, TaskStatus};

use super::{VerifierTask, VERIFIER_QUANTUM};
use crate::{agent_cpu::PreemptedAgentCpu, timer_task_flow::CompletedWorkerTasks, X86BootedKernel};

pub(super) use result::{complete, inspect, verify};

pub(super) fn dispatch(
    booted: &mut X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
) -> Option<RunQueueEntry> {
    if workers.subject() != verifier.subject
        || !workers.both_completed(booted)
        || !booted.kernel().run_queue().is_empty()
    {
        return None;
    }
    let queued = booted
        .kernel_mut()
        .sys_enqueue_task(verifier.agent, verifier.task)
        .ok()?;
    if queued.kind != EventKind::TaskQueued
        || queued.task != Some(verifier.task)
        || booted.kernel().run_queue()
            != [RunQueueEntry {
                task: verifier.task,
                agent: verifier.agent,
            }]
    {
        return None;
    }
    let dispatched = booted
        .kernel_mut()
        .sys_dispatch_next_ready_with_quantum(VERIFIER_QUANTUM)
        .ok()?;
    if dispatched
        != (RunQueueEntry {
            task: verifier.task,
            agent: verifier.agent,
        })
    {
        return None;
    }
    running_state_valid(booted, verifier, workers, 0, EventKind::TaskDispatched)?;
    Some(dispatched)
}

pub(super) fn expire_and_redispatch(
    booted: &mut X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
    cpu: &PreemptedAgentCpu,
) -> Option<()> {
    if cpu.tick_count() != 1 || verifier.call_context() != Some(cpu.context()) {
        return None;
    }
    let expiry = booted
        .kernel_mut()
        .sys_tick_task(verifier.agent, verifier.task)
        .ok()?;
    if expiry.kind != EventKind::TaskQuantumExpired
        || expiry.task != Some(verifier.task)
        || expiry.task_ticks != Some(1)
        || expiry.task_quantum != Some(0)
        || !accepted_after_expiry_valid(booted, verifier, workers)
    {
        return None;
    }
    if booted
        .kernel_mut()
        .sys_dispatch_next_ready_with_quantum(VERIFIER_QUANTUM)
        .ok()?
        != (RunQueueEntry {
            task: verifier.task,
            agent: verifier.agent,
        })
    {
        return None;
    }
    running_state_valid(booted, verifier, workers, 1, EventKind::TaskDispatched)
}

pub(super) fn running_state_valid(
    booted: &X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
    ticks: u64,
    event_kind: EventKind,
) -> Option<()> {
    let kernel = booted.kernel();
    let task = kernel
        .tasks()
        .iter()
        .find(|task| task.id == verifier.task)?;
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == verifier.agent)?;
    let event = kernel.events().last()?;
    (workers.both_completed(booted)
        && task.status == TaskStatus::Running
        && task.run_ticks == ticks
        && task.quantum_remaining == VERIFIER_QUANTUM
        && task.result.is_none()
        && context.state == AgentExecutionState::Running
        && context.task == Some(verifier.task)
        && context.run_ticks == ticks
        && context.quantum_remaining == VERIFIER_QUANTUM
        && kernel.run_queue().is_empty()
        && event.kind == event_kind)
        .then_some(())
}

fn accepted_after_expiry_valid(
    booted: &X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == verifier.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == verifier.agent);
    workers.both_completed(booted)
        && matches!(task, Some(task) if task.status == TaskStatus::Accepted
            && task.run_ticks == 1 && task.quantum_remaining == 0 && task.result.is_none())
        && matches!(context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none())
        && kernel.run_queue()
            == [RunQueueEntry {
                task: verifier.task,
                agent: verifier.agent,
            }]
}
