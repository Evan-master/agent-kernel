//! Terminal proof for Fault Worker live-memory reclamation.
//!
//! This boot-evidence child binds the first fault transcript to the delegated
//! Memory authority, exact cell descriptor, physical proof words, ordered
//! retirement event, empty Agent ledgers, and zeroed global pool state.

use agent_kernel_core::{
    EventKind, MemoryCellId, MemoryValue, Operation, OperationSet, ResourceKind, ResourceStatus,
};
use agent_kernel_x86_64::{
    runtime_reclamation::RuntimeMemoryKind,
    runtime_region::RUNTIME_MEMORY_ACCESS_READ_WRITE,
    user_memory::{UserMemoryLayout, PAGE_BYTES},
};

use crate::{
    agent_cpu::FaultedAgentCpu,
    agent_memory::RuntimeMemoryPool,
    boot_agent_images::BootFaultWorkerImage,
    fault_task_flow::{
        PreparedFaultTaskFlow, FAULT_WORKER, FAULT_WORKER_MEMORY_FIRST_PROOF,
        FAULT_WORKER_MEMORY_LAST_PROOF,
    },
    X86BootedKernel,
};

pub(super) fn valid(
    booted: &X86BootedKernel,
    memory_pool: &RuntimeMemoryPool,
    faulted: &FaultedAgentCpu,
    flow: &PreparedFaultTaskFlow,
    image: BootFaultWorkerImage,
) -> bool {
    let kernel = booted.kernel();
    let resource = kernel
        .resources()
        .iter()
        .find(|record| record.id == flow.memory_resource());
    let root = kernel.capability(flow.memory_root()).ok();
    let capability = kernel.capability(flow.memory_capability()).ok();
    let cell = kernel
        .memory_cells()
        .iter()
        .find(|record| record.id == MemoryCellId::new(1));
    let log = faulted.reclamation_log();
    let expected_descriptor = MemoryValue::new([
        UserMemoryLayout::fixed().runtime_region_start(),
        PAGE_BYTES * 2,
        RUNTIME_MEMORY_ACCESS_READ_WRITE,
        1,
    ]);

    faulted.call_nonce() == Some(image.nonce())
        && faulted.call_count() == 2
        && faulted.operations() == image.first_fault_operations()
        && faulted.return_offsets() == image.first_fault_return_offsets()
        && log.len() == 1
        && log.get(0).is_some_and(|entry| {
            entry.kind() == RuntimeMemoryKind::Region
                && entry.resource() == flow.memory_resource()
                && entry.capability() == flow.memory_capability()
                && entry.cell() == MemoryCellId::new(1)
                && entry.page_count() == 2
                && entry.generation() == 1
                && entry.first() == FAULT_WORKER_MEMORY_FIRST_PROOF
                && entry.last() == FAULT_WORKER_MEMORY_LAST_PROOF
        })
        && matches!(resource, Some(resource)
            if resource.kind == ResourceKind::Memory
                && resource.parent == Some(booted.report().bootstrap_resource)
                && resource.owner == Some(booted.report().bootstrap_agent)
                && resource.status == ResourceStatus::Retired)
        && matches!(root, Some(root)
            if root.agent == booted.report().bootstrap_agent
                && root.resource == flow.memory_resource()
                && root.operations == root_operations()
                && !root.revoked
                && root.task.is_none()
                && root.parent.is_none())
        && matches!(capability, Some(capability)
            if capability.agent == FAULT_WORKER
                && capability.resource == flow.memory_resource()
                && capability.operations == memory_operations()
                && !capability.revoked
                && capability.task.is_none()
                && capability.parent == Some(flow.memory_root()))
        && matches!(cell, Some(cell)
            if cell.resource == flow.memory_resource()
                && cell.creator == FAULT_WORKER
                && cell.last_writer == FAULT_WORKER
                && cell.value == expected_descriptor
                && cell.revision == 1)
        && faulted.runtime_memory_is_clear()
        && memory_pool.agent_is_clear(FAULT_WORKER)
        && memory_pool.all_available_and_zero()
        && events_valid(booted, flow)
}

fn events_valid(booted: &X86BootedKernel, flow: &PreparedFaultTaskFlow) -> bool {
    let events = booted.kernel().events();
    let Some(fault_index) = events
        .iter()
        .position(|event| event.kind == EventKind::TaskFaulted && event.agent == FAULT_WORKER)
    else {
        return false;
    };
    let Some(created) = fault_index
        .checked_sub(2)
        .and_then(|index| events.get(index))
    else {
        return false;
    };
    let Some(retired) = fault_index
        .checked_sub(1)
        .and_then(|index| events.get(index))
    else {
        return false;
    };
    created.kind == EventKind::MemoryCellCreated
        && created.agent == FAULT_WORKER
        && created.resource == Some(flow.memory_resource())
        && created.capability == Some(flow.memory_capability())
        && created.memory_cell == Some(MemoryCellId::new(1))
        && created.operation == Some(Operation::Act)
        && retired.kind == EventKind::ResourceRetired
        && retired.agent == FAULT_WORKER
        && retired.resource == Some(flow.memory_resource())
        && retired.capability == Some(flow.memory_capability())
}

fn memory_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
}

fn root_operations() -> OperationSet {
    memory_operations().with(Operation::Delegate)
}
