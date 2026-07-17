//! Terminal state and event evidence for the native Task Manager protocol.
//!
//! This x86 boot-adapter child reads facade-visible objects and binds the five
//! Manager mutations to exact Intent, Task, capability, and target identities.

use agent_kernel_core::{
    Event, EventKind, IntentKind, IntentStatus, Operation, OperationSet, TaskStatus,
    VerificationRequirement,
};

use super::super::{ResourceManagerTask, RESOURCE_MANAGER};
use crate::{boot_agent_images::BootResourceManagerImage, X86BootedKernel};

pub(super) fn state_valid(
    booted: &X86BootedKernel,
    manager: ResourceManagerTask,
    image: BootResourceManagerImage,
) -> bool {
    let kernel = booted.kernel();
    let intent = kernel
        .intents()
        .iter()
        .find(|record| record.id == image.managed_intent());
    let task = kernel
        .tasks()
        .iter()
        .find(|record| record.id == image.managed_task());
    let capability = kernel.capability(image.task_capability()).ok();

    kernel.intents().len() == 7
        && kernel.tasks().len() == 7
        && matches!(intent, Some(intent)
            if intent.owner == RESOURCE_MANAGER
                && intent.resource == booted.report().bootstrap_resource
                && intent.kind == IntentKind::Act
                && intent.status == IntentStatus::Bound
                && intent.verification == VerificationRequirement::Optional)
        && matches!(task, Some(task)
            if task.intent == image.managed_intent()
                && task.owner == RESOURCE_MANAGER
                && task.resource == booted.report().bootstrap_resource
                && task.assignee == Some(image.target_agent())
                && task.delegated_capability == Some(image.task_capability())
                && task.status == TaskStatus::Delegated
                && task.run_ticks == 0
                && task.quantum_remaining == 0
                && task.last_fault.is_none()
                && task.result.is_none())
        && matches!(capability, Some(capability)
            if capability.agent == image.target_agent()
                && capability.resource == booted.report().bootstrap_resource
                && capability.operations == OperationSet::only(Operation::Act)
                && !capability.revoked
                && capability.task == Some(image.managed_task())
                && capability.parent == Some(manager.resource_authority))
}

pub(super) fn events_valid(
    events: &[Event],
    booted: &X86BootedKernel,
    manager: ResourceManagerTask,
    image: BootResourceManagerImage,
) -> bool {
    let resource = booted.report().bootstrap_resource;
    events.len() == 5
        && events[0].kind == EventKind::IntentDeclared
        && events[0].agent == RESOURCE_MANAGER
        && events[0].resource == Some(resource)
        && events[0].capability == Some(manager.resource_authority)
        && events[0].intent == Some(image.managed_intent())
        && events[0].intent_kind == Some(IntentKind::Act)
        && events[0].operation == Some(Operation::Act)
        && events[0].verification == VerificationRequirement::Optional
        && events[1].kind == EventKind::TaskCreated
        && events[1].agent == RESOURCE_MANAGER
        && events[1].resource == Some(resource)
        && events[1].capability == Some(manager.resource_authority)
        && events[1].intent == Some(image.managed_intent())
        && events[1].task == Some(image.managed_task())
        && events[2].kind == EventKind::IntentBound
        && events[2].agent == RESOURCE_MANAGER
        && events[2].resource == Some(resource)
        && events[2].intent == Some(image.managed_intent())
        && events[2].intent_kind == Some(IntentKind::Act)
        && events[2].task == Some(image.managed_task())
        && events[3].kind == EventKind::CapabilityDerived
        && events[3].agent == RESOURCE_MANAGER
        && events[3].resource == Some(resource)
        && events[3].capability == Some(image.task_capability())
        && events[3].source_capability == Some(manager.resource_authority)
        && events[3].operations == OperationSet::only(Operation::Act)
        && events[3].intent == Some(image.managed_intent())
        && events[3].task == Some(image.managed_task())
        && events[3].target_agent == Some(image.target_agent())
        && events[4].kind == EventKind::DelegationRequested
        && events[4].agent == RESOURCE_MANAGER
        && events[4].resource == Some(resource)
        && events[4].capability == Some(image.task_capability())
        && events[4].intent == Some(image.managed_intent())
        && events[4].task == Some(image.managed_task())
        && events[4].target_agent == Some(image.target_agent())
}
