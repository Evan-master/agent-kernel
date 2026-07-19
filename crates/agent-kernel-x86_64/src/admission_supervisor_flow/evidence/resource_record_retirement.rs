//! Resource Store retirement, monotonic allocation, and slot-reuse proof.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, EventKind, Operation, OperationSet, ResourceId, ResourceKind,
    ResourceStatus,
};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel, X86_RESOURCE_CAPACITY,
};

const AUTHORITY: CapabilityId = CapabilityId::new(23);
const RETIRED_RESOURCE: ResourceId = ResourceId::new(3);
const FRESH_RESOURCE: ResourceId = ResourceId::new(8);
const FRESH_CAPABILITY: CapabilityId = CapabilityId::new(27);
const DELEGATE_CAPABILITY: CapabilityId = CapabilityId::new(28);
const ROLLBACK_CAPABILITY: CapabilityId = CapabilityId::new(29);

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn resource_record_retired_and_reused(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let resources = kernel.resources();
        let events = kernel.events();
        let Some(start) = events.iter().position(|event| {
            event.kind == EventKind::ResourceRecordRetired
                && event.resource == Some(RETIRED_RESOURCE)
        }) else {
            return false;
        };
        let Some(window) = events.get(start..start + 5) else {
            return false;
        };
        let Some(fresh) = resources.iter().find(|record| record.id == FRESH_RESOURCE) else {
            return false;
        };

        resources.len() == X86_RESOURCE_CAPACITY
            && resources
                .iter()
                .map(|record| record.id.raw())
                .eq([1, 2, 5, 6, 7, 8, 9])
            && resources.iter().all(|record| record.id != RETIRED_RESOURCE)
            && resources
                .iter()
                .all(|record| record.id != ResourceId::new(4))
            && fresh.kind == ResourceKind::Service
            && fresh.parent == Some(ResourceId::new(1))
            && fresh.owner == Some(ADMISSION_SUPERVISOR)
            && fresh.status == ResourceStatus::Active
            && fresh_capability_matches(booted)
            && window
                .iter()
                .enumerate()
                .all(|(index, event)| event.sequence == 358 + index as u64)
            && retirement_event_matches(window[0])
            && resource_event_matches(window[1], EventKind::ResourceCreated)
            && resource_event_matches(window[2], EventKind::CapabilityGranted)
            && derived_event_matches(window[3], DELEGATE_CAPABILITY, Operation::Delegate)
            && derived_event_matches(window[4], ROLLBACK_CAPABILITY, Operation::Rollback)
    }
}

fn fresh_capability_matches(booted: &X86BootedKernel) -> bool {
    matches!(booted.kernel().capability(FRESH_CAPABILITY), Ok(capability)
        if capability.agent == ADMISSION_SUPERVISOR
            && capability.resource == FRESH_RESOURCE
            && capability.operations == OperationSet::only(Operation::Observe)
            && !capability.revoked
            && capability.task.is_none()
            && capability.parent.is_none())
}

fn retirement_event_matches(event: Event) -> bool {
    event.kind == EventKind::ResourceRecordRetired
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(RETIRED_RESOURCE)
        && event.capability == Some(AUTHORITY)
        && event.source_capability.is_none()
        && event.operation == Some(Operation::Rollback)
        && event.operations == OperationSet::empty()
        && event.target_agent == Some(AgentId::new(8))
}

fn resource_event_matches(event: Event, kind: EventKind) -> bool {
    event.kind == kind
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(FRESH_RESOURCE)
        && event.capability == Some(FRESH_CAPABILITY)
        && event.source_capability.is_none()
        && event.operation.is_none()
        && event.operations == OperationSet::only(Operation::Observe)
        && event.task.is_none()
        && event.target_agent.is_none()
}

fn derived_event_matches(event: Event, capability: CapabilityId, operation: Operation) -> bool {
    event.kind == EventKind::CapabilityDerived
        && event.agent == ADMISSION_SUPERVISOR
        && event.resource == Some(ResourceId::new(1))
        && event.capability == Some(capability)
        && event.source_capability == Some(AUTHORITY)
        && event.operation.is_none()
        && event.operations == OperationSet::only(operation)
        && event.task.is_none()
        && event.target_agent == Some(ADMISSION_SUPERVISOR)
}
