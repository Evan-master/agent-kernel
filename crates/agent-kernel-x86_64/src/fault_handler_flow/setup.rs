//! Kernel-visible admission and policy binding for the native Fault Handler.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentImageDigest, AgentImageKind, EventKind, FaultKind,
    FaultPolicyAction, IntentKind, RunQueueEntry, TaskStatus, VerificationRequirement,
};

use super::{FaultHandlerTask, FAULT_HANDLER};
use crate::X86BootedKernel;

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    digest: AgentImageDigest,
) -> Option<FaultHandlerTask> {
    let report = *booted.report();
    let kernel = booted.kernel_mut();
    kernel.sys_register_agent(FAULT_HANDLER).ok()?;
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
            FAULT_HANDLER,
        )
        .ok()?;
    let capability = kernel
        .tasks()
        .iter()
        .find(|record| record.id == task)?
        .delegated_capability?;
    let image = kernel
        .sys_register_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            AgentImageKind::FaultHandler,
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
            FAULT_HANDLER,
            capability,
            task,
            image,
            AgentEntryKind::FaultHandler,
        )
        .ok()?;
    kernel.sys_accept_task(FAULT_HANDLER, task).ok()?;
    kernel
        .sys_install_fault_handler(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            FaultKind::ExecutionTrap,
            FAULT_HANDLER,
        )
        .ok()?;
    kernel
        .sys_install_fault_policy(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            FaultKind::ExecutionTrap,
            FaultPolicyAction::RouteToHandler,
        )
        .ok()?;

    let handler = FaultHandlerTask {
        task,
        image,
        capability,
    };
    prepared_state_valid(booted, handler, intent).then_some(handler)
}

fn prepared_state_valid(
    booted: &X86BootedKernel,
    handler: FaultHandlerTask,
    intent: agent_kernel_core::IntentId,
) -> bool {
    let report = *booted.report();
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == handler.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == FAULT_HANDLER);
    let entry = kernel.agent_entry(FAULT_HANDLER).ok();
    let binding = kernel.fault_handlers().first();
    let policy = kernel.fault_policies().first();
    handler.task.raw() == 5
        && handler.image.raw() == 7
        && intent.raw() == 5
        && matches!(task, Some(task)
            if task.status == TaskStatus::Accepted
                && task.delegated_capability == Some(handler.capability)
                && task.run_ticks == 0
                && task.quantum_remaining == 0
                && task.result.is_none())
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Idle && context.task.is_none())
        && matches!(entry, Some(entry)
            if entry.kind == AgentEntryKind::FaultHandler
                && entry.image == handler.image
                && entry.task == Some(handler.task)
                && entry.capability == handler.capability)
        && matches!(binding, Some(binding)
            if binding.resource == report.bootstrap_resource
                && binding.kind == FaultKind::ExecutionTrap
                && binding.installer == report.bootstrap_agent
                && binding.handler == FAULT_HANDLER)
        && matches!(policy, Some(policy)
            if policy.resource == report.bootstrap_resource
                && policy.kind == FaultKind::ExecutionTrap
                && policy.installer == report.bootstrap_agent
                && policy.action == FaultPolicyAction::RouteToHandler)
        && kernel.fault_handlers().len() == 1
        && kernel.fault_policies().len() == 1
        && !kernel.run_queue().contains(&RunQueueEntry {
            task: handler.task,
            agent: FAULT_HANDLER,
        })
        && matches!(kernel.events().last(), Some(event)
            if event.kind == EventKind::FaultPolicyInstalled
                && event.agent == report.bootstrap_agent
                && event.target_agent.is_none())
}
