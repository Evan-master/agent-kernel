//! Read-only semantic, authority, event, and physical proof for Resource Manager V0.

mod agent_management;
mod memory_page;
mod memory_region;
mod task_lifecycle;

use agent_kernel_core::{
    EventKind, Operation, OperationSet, ResourceKind, ResourceStatus, TaskStatus,
};

use super::{ResourceManagerTask, RESOURCE_MANAGER};
use crate::{
    agent_memory::RuntimeMemoryPool, boot_agent_images::BootResourceManagerImage,
    native_agent_executor::NativeExecutionReport, X86BootedKernel,
};

pub(super) fn completed(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    memory_pool: &RuntimeMemoryPool,
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
    let Ok(derived) = kernel.capability(image.derived_capability()) else {
        return false;
    };
    let Ok(authority) = kernel.capability(manager.resource_authority) else {
        return false;
    };
    let Some(region_observation) = completed.runtime_region_observation() else {
        return false;
    };
    let child_operations = OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Delegate)
        .with(Operation::Rollback);

    completed.context() == context
        && completed.nonce() == image.nonce()
        && completed.call_count() == 22
        && completed.address_space_switch_count() == 44
        && completed.operations() == image.expected_operations()
        && completed.return_offsets() == image.expected_return_offsets()
        && completed.physical_quantum_generation() == 1
        && completed.restart_generation() == 0
        && completed.lazy_data_byte() == 0
        && completed.runtime_page_generation() == image.memory_generation()
        && completed.runtime_page_released()
        && completed.runtime_page_observation() == Some(image.memory_proof_value())
        && completed.runtime_region_generation() == image.memory_region_generation()
        && completed.runtime_regions_released()
        && region_observation.first == image.memory_region_first_proof()
        && region_observation.last == image.memory_region_last_proof()
        && region_observation.page_count == image.memory_region_page_count()
        && region_observation.generation == image.memory_region_generation()
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
        && derived.agent == image.target_agent()
        && derived.resource == image.resource()
        && derived.operations == OperationSet::only(Operation::Observe)
        && derived.revoked
        && derived.task.is_none()
        && derived.parent == Some(image.capability())
        && authority.agent == RESOURCE_MANAGER
        && authority.resource == booted.report().bootstrap_resource
        && authority.operations == OperationSet::only(Operation::Act).with(Operation::Delegate)
        && !authority.revoked
        && authority.task.is_none()
        && authority.parent == Some(booted.report().bootstrap_capability)
        && kernel.resources().len() == 4
        && kernel.run_queue().is_empty()
        && memory_pool.all_available_and_zero()
        && task_lifecycle::state_valid(booted, manager, image)
        && agent_management::state_valid(booted, manager, image)
        && memory_page::state_valid(booted, image)
        && memory_region::state_valid(booted, image)
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
        EventKind::CapabilityDerived,
        EventKind::CapabilityRevoked,
        EventKind::ResourceRetired,
        EventKind::IntentDeclared,
        EventKind::TaskCreated,
        EventKind::IntentBound,
        EventKind::CapabilityDerived,
        EventKind::DelegationRequested,
        EventKind::AgentRegistered,
        EventKind::AgentSuspended,
        EventKind::AgentResumed,
        EventKind::AgentRetired,
        EventKind::ResourceCreated,
        EventKind::CapabilityGranted,
        EventKind::MemoryCellCreated,
        EventKind::MemoryCellRecalled,
        EventKind::ResourceRetired,
        EventKind::ResourceCreated,
        EventKind::CapabilityGranted,
        EventKind::MemoryCellCreated,
        EventKind::MemoryCellRecalled,
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
        && tail[6].agent == RESOURCE_MANAGER
        && tail[6].resource == Some(image.resource())
        && tail[6].capability == Some(image.derived_capability())
        && tail[6].source_capability == Some(image.capability())
        && tail[6].operations == OperationSet::only(Operation::Observe)
        && tail[6].target_agent == Some(image.target_agent())
        && tail[7].agent == RESOURCE_MANAGER
        && tail[7].resource == Some(image.resource())
        && tail[7].capability == Some(image.derived_capability())
        && tail[7].source_capability == Some(image.capability())
        && tail[7].operations == OperationSet::only(Operation::Observe)
        && tail[7].target_agent == Some(image.target_agent())
        && tail[8].resource == Some(image.resource())
        && tail[8].capability == Some(image.capability())
        && task_lifecycle::events_valid(&tail[9..14], booted, manager, image)
        && agent_management::events_valid(&tail[14..18], booted, manager, image)
        && memory_page::events_valid(&tail[18..23], booted, image)
        && memory_region::events_valid(&tail[23..28], booted, image)
        && tail[28].task == Some(manager.task)
        && tail[28].task_result == Some(image.result())
        && tail[29].task == Some(manager.task)
}
