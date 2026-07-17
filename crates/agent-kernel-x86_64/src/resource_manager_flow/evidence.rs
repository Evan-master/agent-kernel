//! Read-only semantic, authority, event, and physical proof for Resource Manager V0.

use agent_kernel_core::{
    EventKind, Operation, OperationSet, ResourceKind, ResourceStatus, TaskStatus,
};

use super::{ResourceManagerTask, RESOURCE_MANAGER};
use crate::{
    boot_agent_images::BootResourceManagerImage, native_agent_executor::NativeExecutionReport,
    X86BootedKernel,
};

pub(super) fn completed(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    manager: ResourceManagerTask,
    image: BootResourceManagerImage,
) -> bool {
    let kernel = booted.kernel();
    let Some(completed) = report.completed(RESOURCE_MANAGER) else {
        return false;
    };
    let Some(context) = manager.call_context() else {
        return false;
    };
    let Some(task) = kernel.tasks().iter().find(|task| task.id == manager.task) else {
        return false;
    };
    let Some(resource) = kernel
        .resources()
        .iter()
        .find(|resource| resource.id == image.resource())
    else {
        return false;
    };
    let Ok(capability) = kernel.capability(image.capability()) else {
        return false;
    };
    let Ok(authority) = kernel.capability(manager.resource_authority) else {
        return false;
    };
    let child_operations = OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);

    completed.context() == context
        && completed.nonce() == image.nonce()
        && completed.call_count() == 5
        && completed.address_space_switch_count() == 10
        && completed.operations() == image.expected_operations()
        && completed.return_offsets() == image.expected_return_offsets()
        && completed.physical_quantum_generation() == 1
        && completed.restart_generation() == 0
        && completed.lazy_data_byte() == 0
        && task.status == TaskStatus::Completed
        && task.assignee == Some(RESOURCE_MANAGER)
        && task.delegated_capability == Some(manager.task_capability)
        && task.run_ticks == 1
        && task.result == Some(image.result())
        && resource.kind == ResourceKind::Service
        && resource.parent == Some(booted.report().bootstrap_resource)
        && resource.owner == Some(RESOURCE_MANAGER)
        && resource.status == ResourceStatus::Retired
        && capability.agent == RESOURCE_MANAGER
        && capability.resource == image.resource()
        && capability.operations == child_operations
        && !capability.revoked
        && capability.task.is_none()
        && capability.parent.is_none()
        && authority.agent == RESOURCE_MANAGER
        && authority.resource == booted.report().bootstrap_resource
        && authority.operations == OperationSet::only(Operation::Act)
        && !authority.revoked
        && authority.task.is_none()
        && authority.parent == Some(booted.report().bootstrap_capability)
        && kernel.resources().len() == 2
        && kernel.run_queue().is_empty()
        && events_prove_lifecycle(booted, manager, image)
}

fn events_prove_lifecycle(
    booted: &X86BootedKernel,
    manager: ResourceManagerTask,
    image: BootResourceManagerImage,
) -> bool {
    let expected = [
        EventKind::TaskQueued,
        EventKind::TaskDispatched,
        EventKind::TaskQuantumExpired,
        EventKind::TaskDispatched,
        EventKind::ResourceCreated,
        EventKind::CapabilityGranted,
        EventKind::ResourceRetired,
        EventKind::TaskResultSubmitted,
        EventKind::TaskCompleted,
    ];
    let events = booted.kernel().events();
    let Some(tail) = events.get(events.len().saturating_sub(expected.len())..) else {
        return false;
    };
    tail.iter().map(|event| event.kind).eq(expected)
        && tail[..4]
            .iter()
            .all(|event| event.agent == RESOURCE_MANAGER && event.task == Some(manager.task))
        && tail[2].task_ticks == Some(1)
        && tail[2].task_quantum == Some(0)
        && tail[4].agent == RESOURCE_MANAGER
        && tail[4].resource == Some(image.resource())
        && tail[4].capability == Some(image.capability())
        && tail[5].resource == Some(image.resource())
        && tail[5].capability == Some(image.capability())
        && tail[6].resource == Some(image.resource())
        && tail[6].capability == Some(image.capability())
        && tail[7].task == Some(manager.task)
        && tail[7].task_result == Some(image.result())
        && tail[8].task == Some(manager.task)
}
