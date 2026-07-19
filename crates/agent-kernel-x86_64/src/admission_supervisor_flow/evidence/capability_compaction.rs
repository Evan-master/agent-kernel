//! Sparse Capability Store reuse and lifecycle audit proof.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, EventKind, KernelError, Operation, OperationSet, ResourceId,
};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel, X86_CAPABILITY_CAPACITY,
};

const AUTHORITY: CapabilityId = CapabilityId::new(23);
const RETIRED_RESOURCE_CAPABILITY: CapabilityId = CapabilityId::new(14);
const TRANSIENT_CAPABILITY: CapabilityId = CapabilityId::new(26);
const FIRST_REUSED_CAPABILITY: CapabilityId = CapabilityId::new(27);
const SECOND_REUSED_CAPABILITY: CapabilityId = CapabilityId::new(28);

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn capability_store_compacted(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let report = *booted.report();
        let events = kernel.events();
        let Some(first_compaction) = events.iter().position(|event| {
            event.kind == EventKind::CapabilityCompacted
                && event.capability == Some(RETIRED_RESOURCE_CAPABILITY)
        }) else {
            return false;
        };
        let Some(window_start) = first_compaction.checked_sub(2) else {
            return false;
        };
        let Some(window) = events.get(window_start..window_start + 6) else {
            return false;
        };

        kernel.capability_capacity() == X86_CAPABILITY_CAPACITY
            && kernel.capability_count() == X86_CAPABILITY_CAPACITY
            && kernel.capability(RETIRED_RESOURCE_CAPABILITY)
                == Err(KernelError::CapabilityNotFound)
            && kernel.capability(TRANSIENT_CAPABILITY) == Err(KernelError::CapabilityNotFound)
            && retained_capability_matches(
                kernel.capability(FIRST_REUSED_CAPABILITY).ok(),
                FIRST_REUSED_CAPABILITY,
                report.bootstrap_resource,
                Operation::Delegate,
            )
            && retained_capability_matches(
                kernel.capability(SECOND_REUSED_CAPABILITY).ok(),
                SECOND_REUSED_CAPABILITY,
                report.bootstrap_resource,
                Operation::Rollback,
            )
            && kernel.capability(AUTHORITY).is_ok()
            && events
                .iter()
                .filter(|event| event.kind == EventKind::CapabilityCompacted)
                .count()
                == 2
            && window
                .iter()
                .enumerate()
                .all(|(index, event)| event.sequence == window[0].sequence + index as u64)
            && event_matches(
                window[0],
                EventKind::CapabilityDerived,
                TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                None,
            )
            && event_matches(
                window[1],
                EventKind::CapabilityRevoked,
                TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                None,
            )
            && event_matches(
                window[2],
                EventKind::CapabilityCompacted,
                RETIRED_RESOURCE_CAPABILITY,
                ResourceId::new(3),
                OperationSet::only(Operation::Observe),
                Some(Operation::Rollback),
            )
            && event_matches(
                window[3],
                EventKind::CapabilityCompacted,
                TRANSIENT_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                Some(Operation::Rollback),
            )
            && event_matches(
                window[4],
                EventKind::CapabilityDerived,
                FIRST_REUSED_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Delegate),
                None,
            )
            && event_matches(
                window[5],
                EventKind::CapabilityDerived,
                SECOND_REUSED_CAPABILITY,
                report.bootstrap_resource,
                OperationSet::only(Operation::Rollback),
                None,
            )
    }
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

fn event_matches(
    event: Event,
    kind: EventKind,
    capability: CapabilityId,
    resource: ResourceId,
    operations: OperationSet,
    operation: Option<Operation>,
) -> bool {
    event.kind == kind
        && event.agent == ADMISSION_SUPERVISOR
        && event.capability == Some(capability)
        && event.source_capability == Some(AUTHORITY)
        && event.resource == Some(resource)
        && event.operations == operations
        && event.operation == operation
        && event.task.is_none()
        && event.target_agent
            == Some(if capability == RETIRED_RESOURCE_CAPABILITY {
                AgentId::new(2)
            } else {
                ADMISSION_SUPERVISOR
            })
}
