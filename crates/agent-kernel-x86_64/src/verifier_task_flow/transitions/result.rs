//! Audited result inspection, subject verification, and Verifier completion.

use agent_kernel_core::{AgentExecutionState, EventKind, IntentStatus, TaskStatus};

use super::running_state_valid;
use crate::{
    agent_cpu::{
        AcknowledgedTaskInspectionCpu, AcknowledgedTaskVerificationCpu, CompletedVerifierCpu,
        RequestedTaskInspectionCpu, RequestedTaskVerificationCpu,
    },
    timer_task_flow::CompletedWorkerTasks,
    verifier_task_flow::{subject_result, VerifierTask, VERIFIER_QUANTUM},
    X86BootedKernel,
};

pub(crate) fn inspect(
    booted: &mut X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
    cpu: RequestedTaskInspectionCpu,
) -> Option<AcknowledgedTaskInspectionCpu> {
    if cpu.call_count() != 2
        || cpu.address_space_switch_count() != 4
        || verifier.call_context() != Some(cpu.context())
        || cpu.target() != verifier.subject.task()
    {
        return None;
    }
    let event = booted
        .kernel_mut()
        .sys_inspect_task_result(
            verifier.agent,
            verifier.verify_capability,
            verifier.subject.task(),
        )
        .ok()?;
    if event.kind != EventKind::TaskResultInspected
        || event.agent != verifier.agent
        || event.capability != Some(verifier.verify_capability)
        || event.task != Some(verifier.subject.task())
        || event.task_result != Some(subject_result(verifier))
        || running_state_valid(booted, verifier, workers, 1, EventKind::TaskResultInspected)
            .is_none()
    {
        return None;
    }
    cpu.acknowledge(event.task_result?)
}

pub(crate) fn verify(
    booted: &mut X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
    cpu: RequestedTaskVerificationCpu,
) -> Option<AcknowledgedTaskVerificationCpu> {
    if cpu.call_count() != 3
        || cpu.address_space_switch_count() != 6
        || verifier.call_context() != Some(cpu.context())
        || cpu.target() != verifier.subject.task()
        || cpu.result() != Some(subject_result(verifier))
    {
        return None;
    }
    let event = booted
        .kernel_mut()
        .sys_verify_task(
            verifier.agent,
            verifier.verify_capability,
            verifier.subject.task(),
        )
        .ok()?;
    if event.kind != EventKind::TaskVerified
        || event.agent != verifier.agent
        || event.task != Some(verifier.subject.task())
        || event.capability != Some(verifier.verify_capability)
        || !verified_state_valid(booted, verifier, workers)
    {
        return None;
    }
    cpu.acknowledge()
}

pub(crate) fn complete(
    booted: &mut X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
    cpu: CompletedVerifierCpu,
) -> Option<()> {
    if cpu.call_count() != 4
        || cpu.address_space_switch_count() != 8
        || verifier.call_context() != Some(cpu.context())
        || cpu.target() != verifier.subject.task()
        || cpu.result() != subject_result(verifier)
    {
        return None;
    }
    let event = booted
        .kernel_mut()
        .sys_complete_task(verifier.agent, verifier.task_capability, verifier.task)
        .ok()?;
    (event.kind == EventKind::TaskCompleted
        && event.agent == verifier.agent
        && event.task == Some(verifier.task)
        && event.capability == Some(verifier.task_capability)
        && completed_state_valid(booted, verifier, workers))
    .then_some(())
}

fn verified_state_valid(
    booted: &X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
) -> bool {
    let kernel = booted.kernel();
    let target = kernel
        .tasks()
        .iter()
        .find(|task| task.id == verifier.subject.task());
    let intent = target.and_then(|task| {
        kernel
            .intents()
            .iter()
            .find(|intent| intent.id == task.intent)
    });
    matches!(target, Some(task) if task.status == TaskStatus::Verified
        && task.result == Some(subject_result(verifier)))
        && matches!(intent, Some(intent) if intent.status == IntentStatus::Fulfilled)
        && workers.peer_completed(booted)
        && own_running_valid(booted, verifier)
        && matches!(kernel.events().last(), Some(event) if event.kind == EventKind::IntentFulfilled
            && event.task == Some(verifier.subject.task()))
}

fn completed_state_valid(
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
    matches!(task, Some(task) if task.status == TaskStatus::Completed
        && task.run_ticks == 1 && task.quantum_remaining == VERIFIER_QUANTUM
        && task.result.is_none())
        && matches!(context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none())
        && workers.peer_completed(booted)
        && subject_verified(booted, verifier)
        && kernel.run_queue().is_empty()
}

fn own_running_valid(booted: &X86BootedKernel, verifier: VerifierTask) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == verifier.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == verifier.agent);
    matches!(task, Some(task) if task.status == TaskStatus::Running
        && task.run_ticks == 1 && task.quantum_remaining == VERIFIER_QUANTUM)
        && matches!(context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(verifier.task))
}

fn subject_verified(booted: &X86BootedKernel, verifier: VerifierTask) -> bool {
    let kernel = booted.kernel();
    let target = kernel
        .tasks()
        .iter()
        .find(|task| task.id == verifier.subject.task());
    matches!(target, Some(task) if task.status == TaskStatus::Verified
        && task.result == Some(subject_result(verifier)))
}
