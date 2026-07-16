//! Verifier admission with split execution and verification authority.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentImageDigest, AgentImageKind, EventKind, IntentKind,
    IntentStatus, Operation, OperationSet, TaskStatus, VerificationRequirement,
};

use super::{VerificationSubject, VerifierTask, VERIFIER};
use crate::X86BootedKernel;

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    subject: VerificationSubject,
    digest: AgentImageDigest,
) -> Option<VerifierTask> {
    let report = *booted.report();
    let kernel = booted.kernel_mut();
    kernel.sys_register_agent(VERIFIER).ok()?;
    let intent = kernel
        .sys_declare_intent(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            IntentKind::Verify,
            VerificationRequirement::Optional,
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
            VERIFIER,
        )
        .ok()?;
    let task_capability = kernel
        .tasks()
        .iter()
        .find(|record| record.id == task)?
        .delegated_capability?;
    let verify_capability = kernel
        .sys_derive_capability(
            report.bootstrap_agent,
            report.bootstrap_capability,
            VERIFIER,
            OperationSet::only(Operation::Verify),
        )
        .ok()?;
    let image = kernel
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::Verifier,
            digest,
            1,
            1,
        )
        .ok()?;
    kernel
        .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
        .ok()?;
    kernel
        .sys_launch_task_agent(
            VERIFIER,
            task_capability,
            task,
            image,
            AgentEntryKind::Verifier,
        )
        .ok()?;
    kernel.sys_accept_task(VERIFIER, task).ok()?;

    let verifier = VerifierTask {
        agent: VERIFIER,
        task,
        image,
        task_capability,
        verify_capability,
        subject,
    };
    prepared_state_valid(booted, verifier, intent).then_some(verifier)
}

fn prepared_state_valid(
    booted: &X86BootedKernel,
    verifier: VerifierTask,
    intent: agent_kernel_core::IntentId,
) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == verifier.task);
    let target = kernel
        .tasks()
        .iter()
        .find(|task| task.id == verifier.subject.task());
    let verifier_intent = kernel.intents().iter().find(|record| record.id == intent);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == verifier.agent);
    let entry = kernel.agent_entry(verifier.agent).ok();
    matches!(task, Some(task) if task.status == TaskStatus::Accepted
        && task.delegated_capability == Some(verifier.task_capability)
        && task.run_ticks == 0 && task.quantum_remaining == 0 && task.result.is_none())
        && matches!(target, Some(task) if task.status == TaskStatus::Accepted && task.result.is_none())
        && matches!(verifier_intent, Some(intent) if intent.kind == IntentKind::Verify
            && intent.status == IntentStatus::Bound)
        && matches!(context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none() && context.run_ticks == 0 && context.quantum_remaining == 0)
        && matches!(entry, Some(entry) if entry.kind == AgentEntryKind::Verifier
            && entry.image == verifier.image && entry.task == Some(verifier.task)
            && entry.capability == verifier.task_capability)
        && verifier.task != verifier.subject.task()
        && !kernel
            .run_queue()
            .iter()
            .any(|queued| queued.task == verifier.task)
        && matches!(kernel.events().last(), Some(event) if event.kind == EventKind::TaskAccepted
            && event.agent == verifier.agent && event.task == Some(verifier.task))
}
