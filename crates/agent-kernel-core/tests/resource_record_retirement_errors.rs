mod resource_record_retirement_support;

use agent_kernel_core::{AgentEntryKind, KernelError, Operation, OperationSet, ResourceStatus};

use resource_record_retirement_support::{prepare_target, setup};

#[test]
fn active_resource_rejects_cleanup_and_record_retirement() {
    let (mut core, fixture) = setup::<32>(AgentEntryKind::Supervisor);
    let resources = core.resources().to_vec();
    let events = core.events().len();

    assert_eq!(
        core.revoke_capability_for_cleanup(
            fixture.actor,
            fixture.authority,
            fixture.target.capability,
        ),
        Err(KernelError::CapabilityCleanupNotReady)
    );
    assert_eq!(
        core.retire_resource_record(fixture.actor, fixture.authority, fixture.target.resource),
        Err(KernelError::ResourceRecordRetirementNotReady)
    );
    assert_eq!(core.resources(), resources.as_slice());
    assert_eq!(core.events().len(), events);
    assert_eq!(core.resources()[1].status, ResourceStatus::Active);
}

#[test]
fn cleanup_requires_a_launched_supervisor() {
    let (mut core, fixture) = setup::<32>(AgentEntryKind::Worker);
    core.retire_resource(
        fixture.actor,
        fixture.target.capability,
        fixture.target.resource,
    )
    .unwrap();

    assert_eq!(
        core.revoke_capability_for_cleanup(
            fixture.actor,
            fixture.authority,
            fixture.target.capability,
        ),
        Err(KernelError::AgentEntryKindMismatch)
    );
    assert_eq!(
        core.retire_resource_record(fixture.actor, fixture.authority, fixture.target.resource),
        Err(KernelError::AgentEntryKindMismatch)
    );
}

#[test]
fn event_exhaustion_preserves_the_dense_resource_store() {
    let (mut core, fixture) = setup::<10>(AgentEntryKind::Supervisor);
    prepare_target(&mut core, fixture);
    assert_eq!(core.events().len(), 10);
    let resources = core.resources().to_vec();

    assert_eq!(
        core.retire_resource_record(fixture.actor, fixture.authority, fixture.target.resource),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.resources(), resources.as_slice());
    assert_eq!(core.events().len(), 10);
}

#[test]
fn cleanup_rejects_already_revoked_capability_without_mutation() {
    let (mut core, fixture) = setup::<32>(AgentEntryKind::Supervisor);
    core.retire_resource(
        fixture.actor,
        fixture.target.capability,
        fixture.target.resource,
    )
    .unwrap();
    core.revoke_capability_for_cleanup(fixture.actor, fixture.authority, fixture.target.capability)
        .unwrap();
    let events = core.events().len();

    assert_eq!(
        core.revoke_capability_for_cleanup(
            fixture.actor,
            fixture.authority,
            fixture.target.capability,
        ),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(core.events().len(), events);
}

#[test]
fn cleanup_authority_must_allow_rollback() {
    let (mut core, fixture) = setup::<40>(AgentEntryKind::Supervisor);
    let observe = core
        .grant_capability(
            fixture.actor,
            fixture.root,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();
    core.retire_resource(
        fixture.actor,
        fixture.target.capability,
        fixture.target.resource,
    )
    .unwrap();

    assert_eq!(
        core.revoke_capability_for_cleanup(fixture.actor, observe, fixture.target.capability),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(
        core.retire_resource_record(fixture.actor, observe, fixture.target.resource),
        Err(KernelError::OperationDenied)
    );
}
