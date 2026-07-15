//! Admission and initial FIFO dispatch for two delegated Workers.
//!
//! This child module uses only public kernel syscalls. Both tasks receive
//! attenuated task-scoped capabilities and share one verified Worker image.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageId, AgentImageKind,
    CapabilityId, EventKind, IntentKind, RunQueueEntry, TaskId, TaskStatus,
    VerificationRequirement,
};

use super::{WorkerTask, TASK_QUANTUM, WORKER_A, WORKER_B};
use crate::X86BootedKernel;

#[derive(Copy, Clone)]
struct PendingWorker {
    task: TaskId,
    capability: CapabilityId,
}

pub(super) fn prepare(booted: &mut X86BootedKernel) -> Option<(WorkerTask, WorkerTask)> {
    let first = register_delegated_task(booted, WORKER_A)?;
    let second = register_delegated_task(booted, WORKER_B)?;
    let image = register_worker_image(booted)?;
    launch_and_enqueue(booted, WORKER_A, first, image)?;
    launch_and_enqueue(booted, WORKER_B, second, image)?;

    let kernel = booted.kernel_mut();
    if kernel
        .sys_dispatch_next_with_quantum(WORKER_A, TASK_QUANTUM)
        .ok()?
        != first.task
    {
        return None;
    }
    let first = WorkerTask::new(WORKER_A, first.task);
    let second = WorkerTask::new(WORKER_B, second.task);
    setup_state_valid(booted, first, second).then_some((first, second))
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

fn register_worker_image(booted: &mut X86BootedKernel) -> Option<AgentImageId> {
    let report = *booted.report();
    let kernel = booted.kernel_mut();
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

fn setup_state_valid(booted: &X86BootedKernel, first: WorkerTask, second: WorkerTask) -> bool {
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
    matches!(first_task, Some(task) if task.status == TaskStatus::Running && task.run_ticks == 0 && task.quantum_remaining == TASK_QUANTUM)
        && matches!(second_task, Some(task) if task.status == TaskStatus::Accepted && task.run_ticks == 0 && task.quantum_remaining == 0)
        && matches!(first_context, Some(context) if context.state == AgentExecutionState::Running && context.task == Some(first.task) && context.run_ticks == 0 && context.quantum_remaining == TASK_QUANTUM)
        && matches!(second_context, Some(context) if context.state == AgentExecutionState::Idle && context.task.is_none() && context.run_ticks == 0 && context.quantum_remaining == 0)
        && kernel.run_queue()
            == [RunQueueEntry {
                task: second.task,
                agent: second.agent,
            }]
        && matches!(kernel.events().last(), Some(event) if event.kind == EventKind::TaskDispatched && event.task == Some(first.task))
}
