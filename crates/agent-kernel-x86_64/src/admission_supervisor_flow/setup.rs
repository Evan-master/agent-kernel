//! Least-authority setup for the Runtime Admission Supervisor.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentImageDigest, AgentImageKind, EventKind, IntentKind,
    Operation, OperationSet, TaskStatus, VerificationRequirement,
};

use super::{AdmissionSupervisorTask, ADMISSION_SUPERVISOR};
use crate::X86BootedKernel;

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    digest: AgentImageDigest,
) -> Option<AdmissionSupervisorTask> {
    if !booted.kernel().run_queue().is_empty() {
        return None;
    }
    let report = *booted.report();
    let kernel = booted.kernel_mut();
    kernel.sys_register_agent(ADMISSION_SUPERVISOR).ok()?;
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
            ADMISSION_SUPERVISOR,
        )
        .ok()?;
    let task_capability = kernel
        .tasks()
        .iter()
        .find(|record| record.id == task)?
        .delegated_capability?;
    let admission_authority = kernel
        .sys_derive_capability(
            report.bootstrap_agent,
            report.bootstrap_capability,
            ADMISSION_SUPERVISOR,
            OperationSet::only(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Rollback),
        )
        .ok()?;
    let image = kernel
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::Supervisor,
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
            ADMISSION_SUPERVISOR,
            task_capability,
            task,
            image,
            AgentEntryKind::Supervisor,
        )
        .ok()?;
    kernel.sys_accept_task(ADMISSION_SUPERVISOR, task).ok()?;
    kernel.sys_enqueue_task(ADMISSION_SUPERVISOR, task).ok()?;

    let supervisor = AdmissionSupervisorTask {
        intent,
        task,
        image,
        task_capability,
        admission_authority,
    };
    prepared_state_valid(booted, supervisor).then_some(supervisor)
}

fn prepared_state_valid(booted: &X86BootedKernel, supervisor: AdmissionSupervisorTask) -> bool {
    let report = *booted.report();
    let kernel = booted.kernel();
    let task = kernel
        .tasks()
        .iter()
        .find(|record| record.id == supervisor.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|record| record.agent == ADMISSION_SUPERVISOR);
    let entry = kernel.agent_entry(ADMISSION_SUPERVISOR).ok();
    let authority = kernel.capability(supervisor.admission_authority).ok();
    supervisor.intent.raw() == 10
        && supervisor.task.raw() == 10
        && supervisor.image.raw() == 12
        && supervisor.task_capability.raw() == 22
        && supervisor.admission_authority.raw() == 23
        && matches!(task, Some(task)
            if task.status == TaskStatus::Accepted
                && task.delegated_capability == Some(supervisor.task_capability)
                && task.run_ticks == 0
                && task.result.is_none())
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Idle && context.task.is_none())
        && matches!(entry, Some(entry)
            if entry.kind == AgentEntryKind::Supervisor
                && entry.image == supervisor.image
                && entry.task == Some(supervisor.task)
                && entry.capability == supervisor.task_capability)
        && matches!(authority, Some(authority)
            if authority.agent == ADMISSION_SUPERVISOR
                && authority.resource == report.bootstrap_resource
                && authority.operations
                    == OperationSet::only(Operation::Act)
                        .with(Operation::Delegate)
                        .with(Operation::Rollback)
                && !authority.revoked
                && authority.task.is_none()
                && authority.parent == Some(report.bootstrap_capability))
        && kernel.run_queue() == [supervisor_queue_entry(supervisor)]
        && matches!(kernel.events().last(), Some(event)
            if event.kind == EventKind::TaskQueued
                && event.agent == ADMISSION_SUPERVISOR
                && event.task == Some(supervisor.task))
}

const fn supervisor_queue_entry(
    supervisor: AdmissionSupervisorTask,
) -> agent_kernel_core::RunQueueEntry {
    agent_kernel_core::RunQueueEntry {
        task: supervisor.task,
        agent: ADMISSION_SUPERVISOR,
    }
}
