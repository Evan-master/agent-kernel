//! Capability cleanup, dense compaction, and sparse slot-reuse proof.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, EventKind, KernelError, Operation, OperationSet, ResourceId,
};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel, X86_CAPABILITY_CAPACITY,
};

const AUTHORITY: CapabilityId = CapabilityId::new(23);
const INITIAL_RESOURCE_CAPABILITY: CapabilityId = CapabilityId::new(13);
const DERIVED_RESOURCE_CAPABILITY: CapabilityId = CapabilityId::new(14);
const RETIRED_MEMORY_CAPABILITY: CapabilityId = CapabilityId::new(16);
const RETIRED_REGION_CAPABILITY: CapabilityId = CapabilityId::new(17);
const TRANSIENT_CAPABILITY: CapabilityId = CapabilityId::new(26);
const SECOND_TRANSIENT_CAPABILITY: CapabilityId = CapabilityId::new(27);
const FRESH_RESOURCE_CAPABILITY: CapabilityId = CapabilityId::new(28);
const DELEGATE_CAPABILITY: CapabilityId = CapabilityId::new(29);
const ROLLBACK_CAPABILITY: CapabilityId = CapabilityId::new(30);

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn capability_store_compacted(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let report = *booted.report();
        let events = kernel.events();
        let Some(start) = events.iter().position(|event| {
            event.kind == EventKind::CapabilityDerived
                && event.capability == Some(TRANSIENT_CAPABILITY)
        }) else {
            return false;
        };
        let Some(window) = events.get(start..start + 7) else {
            return false;
        };

        kernel.capability_capacity() == X86_CAPABILITY_CAPACITY
            && kernel.capability_count() == X86_CAPABILITY_CAPACITY
            && missing(booted, RETIRED_MEMORY_CAPABILITY)
            && missing(booted, TRANSIENT_CAPABILITY)
            && missing(booted, SECOND_TRANSIENT_CAPABILITY)
            && initial_resource_capability_matches(booted)
            && derived_resource_capability_matches(booted)
            && retired_region_capability_matches(booted)
            && fresh_resource_capability_matches(booted)
            && retained_capability_matches(
                kernel.capability(DELEGATE_CAPABILITY).ok(),
                DELEGATE_CAPABILITY,
                report.bootstrap_resource,
                Operation::Delegate,
            )
            && retained_capability_matches(
                kernel.capability(ROLLBACK_CAPABILITY).ok(),
                ROLLBACK_CAPABILITY,
                report.bootstrap_resource,
                Operation::Rollback,
            )
            && kernel.capability(AUTHORITY).is_ok()
            && events
                .iter()
                .filter(|event| event.kind == EventKind::CapabilityCompacted)
                .count()
                == 3
            && window
                .iter()
                .enumerate()
                .all(|(index, event)| event.sequence == 357 + index as u64)
            && capability_event_matches(
                window[0],
                EventKind::CapabilityDerived,
                TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                None,
                ADMISSION_SUPERVISOR,
            )
            && capability_event_matches(
                window[1],
                EventKind::CapabilityRevoked,
                TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                None,
                ADMISSION_SUPERVISOR,
            )
            && capability_event_matches(
                window[2],
                EventKind::CapabilityCompacted,
                TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                Some(Operation::Rollback),
                ADMISSION_SUPERVISOR,
            )
            && capability_event_matches(
                window[3],
                EventKind::CapabilityDerived,
                SECOND_TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                None,
                ADMISSION_SUPERVISOR,
            )
            && capability_event_matches(
                window[4],
                EventKind::CapabilityRevoked,
                SECOND_TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                None,
                ADMISSION_SUPERVISOR,
            )
            && capability_event_matches(
                window[5],
                EventKind::CapabilityCompacted,
                SECOND_TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                Some(Operation::Rollback),
                ADMISSION_SUPERVISOR,
            )
            && capability_event_matches(
                window[6],
                EventKind::CapabilityRevoked,
                RETIRED_MEMORY_CAPABILITY,
                ResourceId::new(4),
                memory_operations(),
                Some(Operation::Rollback),
                AgentId::new(8),
            )
    }
}

fn missing(booted: &X86BootedKernel, capability: CapabilityId) -> bool {
    booted.kernel().capability(capability) == Err(KernelError::CapabilityNotFound)
}

fn fresh_resource_capability_matches(booted: &X86BootedKernel) -> bool {
    matches!(booted.kernel().capability(FRESH_RESOURCE_CAPABILITY), Ok(capability)
        if capability.agent == ADMISSION_SUPERVISOR
            && capability.resource == ResourceId::new(8)
            && capability.operations == OperationSet::only(Operation::Observe)
            && !capability.revoked
            && capability.task.is_none()
            && capability.parent.is_none())
}

fn initial_resource_capability_matches(booted: &X86BootedKernel) -> bool {
    matches!(booted.kernel().capability(INITIAL_RESOURCE_CAPABILITY), Ok(capability)
        if capability.agent == AgentId::new(8)
            && capability.resource == ResourceId::new(3)
            && capability.operations == resource_operations()
            && !capability.revoked
            && capability.task.is_none()
            && capability.parent.is_none())
}

fn derived_resource_capability_matches(booted: &X86BootedKernel) -> bool {
    matches!(booted.kernel().capability(DERIVED_RESOURCE_CAPABILITY), Ok(capability)
        if capability.agent == AgentId::new(2)
            && capability.resource == ResourceId::new(3)
            && capability.operations == OperationSet::only(Operation::Observe)
            && capability.revoked
            && capability.task.is_none()
            && capability.parent == Some(INITIAL_RESOURCE_CAPABILITY))
}

fn retired_region_capability_matches(booted: &X86BootedKernel) -> bool {
    matches!(booted.kernel().capability(RETIRED_REGION_CAPABILITY), Ok(capability)
        if capability.agent == AgentId::new(8)
            && capability.resource == ResourceId::new(5)
            && capability.operations == memory_operations()
            && capability.revoked
            && capability.task.is_none()
            && capability.parent.is_none())
}

fn retained_capability_matches(
    capability: Option<agent_kernel_core::Capability>,
    id: CapabilityId,
    resource: ResourceId,
    operation: Operation,
) -> bool {
    matches!(capability, Some(capability)
        if capability.id == id
            && capability.agent == ADMISSION_SUPERVISOR
            && capability.resource == resource
            && capability.operations == OperationSet::only(operation)
            && !capability.revoked
            && capability.task.is_none()
            && capability.parent == Some(AUTHORITY))
}

const fn resource_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

const fn memory_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
}

fn capability_event_matches(
    event: Event,
    kind: EventKind,
    capability: CapabilityId,
    resource: ResourceId,
    operations: OperationSet,
    operation: Option<Operation>,
    target: AgentId,
) -> bool {
    event.kind == kind
        && event.agent == ADMISSION_SUPERVISOR
        && event.capability == Some(capability)
        && event.source_capability == Some(AUTHORITY)
        && event.resource == Some(resource)
        && event.operations == operations
        && event.operation == operation
        && event.task.is_none()
        && event.target_agent == Some(target)
}
