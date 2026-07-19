//! Terminal state and event evidence for the native Agent Manager protocol.
//!
//! This boot-evidence child binds the four ring-3 mutations to Agent 9, the
//! Manager's root Resource authority, an idle execution context, and exact
//! ordered lifecycle events.

use agent_kernel_core::{
    AgentExecutionState, AgentStatus, Event, EventKind, MessageKind, MessageStatus, Operation,
    TaskStatus,
};

use super::super::{ResourceManagerTask, RESOURCE_MANAGER};
use crate::{boot_agent_images::BootResourceManagerImage, X86BootedKernel};

pub(super) fn state_valid(
    booted: &X86BootedKernel,
    manager: ResourceManagerTask,
    image: BootResourceManagerImage,
) -> bool {
    let kernel = booted.kernel();
    let agent = kernel
        .agents()
        .iter()
        .find(|record| record.id == image.managed_agent());
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|record| record.agent == image.managed_agent());
    let authority = kernel.capability(manager.resource_authority).ok();

    kernel.agents().len() == 9
        && matches!(agent, Some(agent)
            if agent.status == AgentStatus::Retired
                && agent.manager == Some(RESOURCE_MANAGER)
                && agent.management_resource == Some(booted.report().bootstrap_resource))
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Idle
                && context.task.is_none()
                && context.driver_invocation.is_none()
                && context.run_ticks == 0
                && context.quantum_remaining == 0)
        && !kernel
            .agent_entries()
            .iter()
            .any(|entry| entry.agent == image.managed_agent())
        && !kernel
            .messages()
            .iter()
            .any(|message| message.id == image.orphaned_message())
        && !kernel.tasks().iter().any(|task| {
            task.assignee == Some(image.managed_agent())
                && !matches!(
                    task.status,
                    TaskStatus::Completed | TaskStatus::Verified | TaskStatus::Cancelled
                )
        })
        && matches!(authority, Some(authority)
            if authority.agent == RESOURCE_MANAGER
                && authority.resource == booted.report().bootstrap_resource
                && authority.operations.allows(Operation::Delegate)
                && !authority.revoked
                && authority.task.is_none())
}

pub(super) fn events_valid(
    events: &[Event],
    booted: &X86BootedKernel,
    manager: ResourceManagerTask,
    image: BootResourceManagerImage,
) -> bool {
    let lifecycle_kinds = [
        (0, EventKind::AgentRegistered),
        (2, EventKind::AgentSuspended),
        (3, EventKind::AgentResumed),
        (4, EventKind::AgentRetired),
    ];
    events.len() == 6
        && lifecycle_kinds.iter().all(|(index, kind)| {
            let event = events[*index];
            event.kind == *kind
                && event.agent == RESOURCE_MANAGER
                && event.target_agent == Some(image.managed_agent())
                && event.resource == Some(booted.report().bootstrap_resource)
                && event.capability == Some(manager.resource_authority)
                && event.operation == Some(Operation::Delegate)
        })
        && events[1].kind == EventKind::MessageSent
        && events[1].agent == RESOURCE_MANAGER
        && events[1].target_agent == Some(image.managed_agent())
        && events[1].message == Some(image.orphaned_message())
        && events[5].kind == EventKind::OrphanedMessageRetired
        && events[5].agent == RESOURCE_MANAGER
        && events[5].target_agent == Some(image.managed_agent())
        && events[5].message == Some(image.orphaned_message())
        && events[5].message_kind == Some(MessageKind::Notify)
        && events[5].source_capability == Some(manager.resource_authority)
        && events[5].operation == Some(Operation::Delegate)
        && events[5].resource.is_none()
        && events[5].capability.is_none()
        && events[5].intent.is_none()
        && events[5].task.is_none()
        && events[5].action.is_none()
        && events[5].fault.is_none()
        && !booted.kernel().messages().iter().any(|message| {
            message.id == image.orphaned_message() || message.status == MessageStatus::Pending
        })
}
