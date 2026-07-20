//! MemoryCell, authority, Resource, Event, and physical reuse proof.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, EventKind, KernelError, MemoryCellId, MemoryValue, Operation,
    OperationSet, ResourceId, ResourceKind, ResourceStatus,
};
use agent_kernel_x86_64::{
    runtime_page::RUNTIME_PAGE_ACCESS_READ_WRITE,
    runtime_reclamation::RuntimeMemoryKind,
    user_memory::{UserMemoryLayout, PAGE_BYTES},
};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    native_agent_executor::NativeExecutionReport,
    X86BootedKernel, X86_MEMORY_CELL_CAPACITY,
};

const AUTHORITY: CapabilityId = CapabilityId::new(23);
const RETIRED_CELL: MemoryCellId = MemoryCellId::new(2);
const RETIRED_RESOURCE: ResourceId = ResourceId::new(4);
const RETIRED_CAPABILITY: CapabilityId = CapabilityId::new(16);
const FRESH_CELL: MemoryCellId = MemoryCellId::new(6);
const FRESH_RESOURCE: ResourceId = ResourceId::new(9);
const FRESH_CAPABILITY: CapabilityId = CapabilityId::new(30);
const MEMORY_PROOF: u64 = 0x4d45_4d43_454c_4c36;

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn memory_cell_record_retired_and_reused(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
    ) -> bool {
        let kernel = booted.kernel();
        let cells = kernel.memory_cells();
        let Some(cell) = cells.iter().find(|record| record.id == FRESH_CELL) else {
            return false;
        };
        let Some(resource) = kernel
            .resources()
            .iter()
            .find(|record| record.id == FRESH_RESOURCE)
        else {
            return false;
        };
        let Ok(capability) = kernel.capability(FRESH_CAPABILITY) else {
            return false;
        };
        let Some(completed) = report.completed(ADMISSION_SUPERVISOR) else {
            return false;
        };
        let reclamation = completed.reclamation_log();
        let Some(reclaimed) = reclamation.get(0) else {
            return false;
        };
        let Some(start) = kernel
            .events()
            .iter()
            .position(|event| event.sequence == 368)
        else {
            return false;
        };
        let Some(events) = kernel.events().get(start..start + 10) else {
            return false;
        };

        cells.len() == X86_MEMORY_CELL_CAPACITY
            && cells
                .iter()
                .map(|record| record.id.raw())
                .eq([1, 3, 4, 5, 6])
            && cells.iter().all(|record| record.id != RETIRED_CELL)
            && cell.resource == FRESH_RESOURCE
            && cell.creator == ADMISSION_SUPERVISOR
            && cell.last_writer == ADMISSION_SUPERVISOR
            && cell.value == memory_descriptor()
            && cell.revision == 1
            && resource.kind == ResourceKind::Memory
            && resource.parent == Some(ResourceId::new(1))
            && resource.owner == Some(ADMISSION_SUPERVISOR)
            && resource.status == ResourceStatus::Retired
            && capability.agent == ADMISSION_SUPERVISOR
            && capability.resource == FRESH_RESOURCE
            && capability.operations == memory_operations()
            && !capability.revoked
            && capability.task.is_none()
            && capability.parent.is_none()
            && kernel.capability(RETIRED_CAPABILITY) == Err(KernelError::CapabilityNotFound)
            && kernel
                .resources()
                .iter()
                .all(|record| record.id != RETIRED_RESOURCE)
            && completed.runtime_page_generation() == 1
            && completed.runtime_page_released()
            && completed.runtime_page_observation().is_none()
            && reclamation.len() == 1
            && reclaimed.kind() == RuntimeMemoryKind::Page
            && reclaimed.resource() == FRESH_RESOURCE
            && reclaimed.capability() == FRESH_CAPABILITY
            && reclaimed.cell() == FRESH_CELL
            && reclaimed.page_count() == 1
            && reclaimed.generation() == 1
            && reclaimed.first() == MEMORY_PROOF
            && reclaimed.last() == MEMORY_PROOF
            && events
                .iter()
                .enumerate()
                .all(|(index, event)| event.sequence == 368 + index as u64)
            && retirement_event(events[0])
            && capability_event(events[1], EventKind::CapabilityRevoked)
            && capability_event(events[2], EventKind::CapabilityCompacted)
            && resource_retirement_event(events[3])
            && resource_event(events[4], EventKind::ResourceCreated)
            && resource_event(events[5], EventKind::CapabilityGranted)
            && memory_creation_event(events[6])
            && task_event(
                events[7],
                EventKind::TaskResultSubmitted,
                self.supervisor.task,
            )
            && resource_reclaimed_event(events[8])
            && task_event(events[9], EventKind::TaskCompleted, self.supervisor.task)
    }
}

const fn memory_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
}

fn memory_descriptor() -> MemoryValue {
    MemoryValue::new([
        UserMemoryLayout::fixed().runtime_page_start(),
        PAGE_BYTES,
        RUNTIME_PAGE_ACCESS_READ_WRITE,
        1,
    ])
}

fn retirement_event(event: Event) -> bool {
    event.kind == EventKind::MemoryCellRecordRetired
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(RETIRED_RESOURCE)
        && event.capability == Some(AUTHORITY)
        && event.source_capability.is_none()
        && event.memory_cell == Some(RETIRED_CELL)
        && event.operation == Some(Operation::Rollback)
        && event.operations == OperationSet::empty()
        && event.target_agent == Some(AgentId::new(8))
}

fn capability_event(event: Event, kind: EventKind) -> bool {
    event.kind == kind
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(RETIRED_RESOURCE)
        && event.capability == Some(RETIRED_CAPABILITY)
        && event.source_capability == Some(AUTHORITY)
        && event.operation == Some(Operation::Rollback)
        && event.operations == memory_operations()
        && event.target_agent == Some(AgentId::new(8))
}

fn resource_retirement_event(event: Event) -> bool {
    event.kind == EventKind::ResourceRecordRetired
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(RETIRED_RESOURCE)
        && event.capability == Some(AUTHORITY)
        && event.source_capability.is_none()
        && event.operation == Some(Operation::Rollback)
        && event.target_agent == Some(AgentId::new(8))
}

fn resource_event(event: Event, kind: EventKind) -> bool {
    event.kind == kind
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(FRESH_RESOURCE)
        && event.capability == Some(FRESH_CAPABILITY)
        && event.source_capability.is_none()
        && event.operation.is_none()
        && event.operations == memory_operations()
        && event.target_agent.is_none()
}

fn memory_creation_event(event: Event) -> bool {
    event.kind == EventKind::MemoryCellCreated
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(FRESH_RESOURCE)
        && event.capability == Some(FRESH_CAPABILITY)
        && event.memory_cell == Some(FRESH_CELL)
        && event.operation == Some(Operation::Act)
        && event.operations == OperationSet::empty()
        && event.target_agent.is_none()
}

fn resource_reclaimed_event(event: Event) -> bool {
    event.kind == EventKind::ResourceRetired
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(FRESH_RESOURCE)
        && event.capability == Some(FRESH_CAPABILITY)
}

fn task_event(event: Event, kind: EventKind, task: agent_kernel_core::TaskId) -> bool {
    event.kind == kind && event.agent == ADMISSION_SUPERVISOR && event.task == Some(task)
}
