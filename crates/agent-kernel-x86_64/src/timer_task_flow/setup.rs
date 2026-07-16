//! Admission and queued state for two delegated Worker images.
//!
//! This child module uses only public kernel syscalls. Both tasks receive
//! attenuated task-scoped capabilities and remain queued until both capsules
//! have been verified and loaded by the architecture adapter.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageId, AgentImageKind,
    CapabilityId, EventKind, IntentKind, RunQueueEntry, TaskId, TaskResult, TaskStatus,
    VerificationRequirement,
};

use super::{WorkerTask, TASK_QUANTUM, WORKER_A, WORKER_B};
use crate::X86BootedKernel;

#[derive(Copy, Clone)]
struct PendingWorker {
    task: TaskId,
    capability: CapabilityId,
}

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    first_digest: AgentImageDigest,
    second_digest: AgentImageDigest,
    first_result: TaskResult,
    second_result: TaskResult,
) -> Option<(WorkerTask, WorkerTask)> {
    let first = register_delegated_task(booted, WORKER_A)?;
    let second = register_delegated_task(booted, WORKER_B)?;
    let first_image = register_worker_image(booted, first_digest)?;
    let second_image = register_worker_image(booted, second_digest)?;
    launch_and_enqueue(booted, WORKER_B, second, second_image)?;
    launch_and_enqueue(booted, WORKER_A, first, first_image)?;

    let first = WorkerTask::new(
        WORKER_A,
        first.task,
        first_image,
        first.capability,
        first_result,
    );
    let second = WorkerTask::new(
        WORKER_B,
        second.task,
        second_image,
        second.capability,
        second_result,
    );
    queued_state_valid(booted, first, second).then_some((first, second))
}

fn register_delegated_task(booted: &mut X86BootedKernel, worker: AgentId) -> Option<PendingWorker> {
    let report = *booted.report();
    let kernel = booted.kernel_mut();
    kernel.sys_register_agent(worker).ok()?;
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
            worker,
        )
        .ok()?;
    let capability = kernel
        .tasks()
        .iter()
        .find(|record| record.id == task)?
        .delegated_capability?;
    Some(PendingWorker { task, capability })
}

fn register_worker_image(
    booted: &mut X86BootedKernel,
    digest: AgentImageDigest,
) -> Option<AgentImageId> {
    let report = *booted.report();
    let kernel = booted.kernel_mut();
    let image = kernel
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::Worker,
            digest,
            1,
            1,
        )
        .ok()?;
    kernel
        .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
        .ok()?;
    Some(image)
}

fn launch_and_enqueue(
    booted: &mut X86BootedKernel,
    worker: AgentId,
    pending: PendingWorker,
    image: AgentImageId,
) -> Option<()> {
    let kernel = booted.kernel_mut();
    kernel
        .sys_launch_task_agent(
            worker,
            pending.capability,
            pending.task,
            image,
            AgentEntryKind::Worker,
        )
        .ok()?;
    kernel.sys_accept_task(worker, pending.task).ok()?;
    kernel.sys_enqueue_task(worker, pending.task).ok()?;
    Some(())
}

fn queued_state_valid(booted: &X86BootedKernel, first: WorkerTask, second: WorkerTask) -> bool {
    let kernel = booted.kernel();
    let first_task = kernel.tasks().iter().find(|task| task.id == first.task);
    let second_task = kernel.tasks().iter().find(|task| task.id == second.task);
    let first_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == first.agent);
    let second_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == second.agent);
    let first_entry = kernel.agent_entry(first.agent).ok();
    let second_entry = kernel.agent_entry(second.agent).ok();
    matches!(first_task, Some(task) if task.status == TaskStatus::Accepted && task.run_ticks == 0 && task.quantum_remaining == 0 && task.delegated_capability == Some(first.capability) && task.result.is_none())
        && matches!(second_task, Some(task) if task.status == TaskStatus::Accepted && task.run_ticks == 0 && task.quantum_remaining == 0 && task.delegated_capability == Some(second.capability) && task.result.is_none())
        && matches!(first_context, Some(context) if context.state == AgentExecutionState::Idle && context.task.is_none() && context.run_ticks == 0 && context.quantum_remaining == 0)
        && matches!(second_context, Some(context) if context.state == AgentExecutionState::Idle && context.task.is_none() && context.run_ticks == 0 && context.quantum_remaining == 0)
        && matches!(first_entry, Some(entry) if entry.image == first.image && entry.task == Some(first.task) && entry.capability == first.capability)
        && matches!(second_entry, Some(entry) if entry.image == second.image && entry.task == Some(second.task) && entry.capability == second.capability)
        && kernel.run_queue()
            == [
                RunQueueEntry {
                    task: second.task,
                    agent: second.agent,
                },
                RunQueueEntry {
                    task: first.task,
                    agent: first.agent,
                },
            ]
        && matches!(kernel.events().last(), Some(event) if event.kind == EventKind::TaskQueued && event.task == Some(first.task))
}

pub(super) fn dispatch_second(
    booted: &mut X86BootedKernel,
    first: WorkerTask,
    second: WorkerTask,
) -> Option<RunQueueEntry> {
    let dispatched = booted
        .kernel_mut()
        .sys_dispatch_next_ready_with_quantum(TASK_QUANTUM)
        .ok()?;
    if dispatched
        != (RunQueueEntry {
            task: second.task,
            agent: second.agent,
        })
    {
        return None;
    }
    let kernel = booted.kernel();
    let first_task = kernel.tasks().iter().find(|task| task.id == first.task)?;
    let first_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == first.agent)?;
    let second_task = kernel.tasks().iter().find(|task| task.id == second.task)?;
    let second_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == second.agent)?;
    (first_task.status == TaskStatus::Accepted
        && first_task.run_ticks == 0
        && first_task.quantum_remaining == 0
        && first_task.result.is_none()
        && first_context.state == AgentExecutionState::Idle
        && first_context.task.is_none()
        && first_context.run_ticks == 0
        && first_context.quantum_remaining == 0
        && second_task.status == TaskStatus::Running
        && second_task.run_ticks == 0
        && second_task.quantum_remaining == TASK_QUANTUM
        && second_task.result.is_none()
        && second_context.state == AgentExecutionState::Running
        && second_context.task == Some(second.task)
        && second_context.run_ticks == 0
        && second_context.quantum_remaining == TASK_QUANTUM
        && kernel.run_queue()
            == [RunQueueEntry {
                task: first.task,
                agent: first.agent,
            }]
        && matches!(kernel.events().last(), Some(event) if event.kind == EventKind::TaskDispatched && event.task == Some(second.task)))
    .then_some(dispatched)
}
