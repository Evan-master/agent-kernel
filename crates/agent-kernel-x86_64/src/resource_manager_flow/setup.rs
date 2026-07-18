//! Kernel-visible admission and least-authority binding for the Resource Manager.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentImageDigest, AgentImageKind, EventKind, IntentKind,
    Operation, OperationSet, RunQueueEntry, TaskStatus, VerificationRequirement,
};

use super::{ResourceManagerTask, RESOURCE_MANAGER};
use crate::X86BootedKernel;

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    digest: AgentImageDigest,
) -> Option<ResourceManagerTask> {
    let report = *booted.report();
    let kernel = booted.kernel_mut();
    kernel.sys_register_agent(RESOURCE_MANAGER).ok()?;
    let intent = kernel
        .sys_declare_intent(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            IntentKind::Act,
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
            RESOURCE_MANAGER,
        )
        .ok()?;
    let task_capability = kernel
        .tasks()
        .iter()
        .find(|record| record.id == task)?
        .delegated_capability?;
    let resource_authority = kernel
        .sys_derive_capability(
            report.bootstrap_agent,
            report.bootstrap_capability,
            RESOURCE_MANAGER,
            OperationSet::only(Operation::Act).with(Operation::Delegate),
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
            RESOURCE_MANAGER,
            task_capability,
            task,
            image,
            AgentEntryKind::Supervisor,
        )
        .ok()?;
    kernel.sys_accept_task(RESOURCE_MANAGER, task).ok()?;

    let manager = ResourceManagerTask {
        task,
        image,
        task_capability,
        resource_authority,
    };
    prepared_state_valid(booted, manager, intent).then_some(manager)
}

fn prepared_state_valid(
    booted: &X86BootedKernel,
    manager: ResourceManagerTask,
    intent: agent_kernel_core::IntentId,
) -> bool {
    let report = *booted.report();
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == manager.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == RESOURCE_MANAGER);
    let entry = kernel.agent_entry(RESOURCE_MANAGER).ok();
    let authority = kernel.capability(manager.resource_authority).ok();
    manager.task.raw() == 6
        && manager.image.raw() == 8
        && manager.task_capability.raw() == 11
        && manager.resource_authority.raw() == 12
        && intent.raw() == 6
        && matches!(task, Some(task)
            if task.status == TaskStatus::Accepted
                && task.delegated_capability == Some(manager.task_capability)
                && task.run_ticks == 0
                && task.quantum_remaining == 0
                && task.result.is_none())
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Idle && context.task.is_none())
        && matches!(entry, Some(entry)
            if entry.kind == AgentEntryKind::Supervisor
                && entry.image == manager.image
                && entry.task == Some(manager.task)
                && entry.capability == manager.task_capability)
        && matches!(authority, Some(authority)
            if authority.agent == RESOURCE_MANAGER
                && authority.resource == report.bootstrap_resource
                && authority.operations
                    == OperationSet::only(Operation::Act).with(Operation::Delegate)
                && !authority.revoked
                && authority.task.is_none()
                && authority.parent == Some(report.bootstrap_capability))
        && !kernel.run_queue().contains(&RunQueueEntry {
            task: manager.task,
            agent: RESOURCE_MANAGER,
        })
        && matches!(kernel.events().last(), Some(event)
            if event.kind == EventKind::TaskAccepted
                && event.agent == RESOURCE_MANAGER
                && event.task == Some(manager.task))
}
