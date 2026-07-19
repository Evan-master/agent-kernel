mod resource_record_retirement_support;

use agent_kernel_core::{
    AgentEntryKind, EventKind, KernelError, Operation, OperationSet, ResourceId, ResourceKind,
};

use resource_record_retirement_support::{prepare_target, setup};

#[test]
fn retirement_preserves_dense_order_and_reuses_capacity_with_fresh_ids() {
    let (mut core, fixture) = setup::<64>(AgentEntryKind::Supervisor);
    let trailing = core
        .register_resource(ResourceKind::Memory, Some(fixture.root))
        .expect("trailing resource registers");
    prepare_target(&mut core, fixture);
    let target_record = core
        .resources()
        .iter()
        .find(|record| record.id == fixture.target.resource)
        .copied()
        .unwrap();
    let event_start = core.events().len();

    let receipt = core
        .retire_resource_record(fixture.actor, fixture.authority, fixture.target.resource)
        .expect("terminal unreferenced record retires");

    assert_eq!(receipt.record(), target_record);
    assert_eq!(receipt.resource(), fixture.target.resource);
    assert_eq!(receipt.actor(), fixture.actor);
    assert_eq!(receipt.authority(), fixture.authority);
    assert_eq!(
        core.resources()
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![fixture.root, trailing]
    );
    assert!(!core
        .resources()
        .iter()
        .any(|record| record.id == fixture.target.resource));

    let event = core.events()[event_start];
    assert_eq!(event.kind, EventKind::ResourceRecordRetired);
    assert_eq!(event.agent, fixture.actor);
    assert_eq!(event.resource, Some(fixture.target.resource));
    assert_eq!(event.capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.target_agent, target_record.owner);

    let fresh = core
        .create_resource(
            fixture.actor,
            ResourceKind::Service,
            Some((fixture.root, fixture.authority)),
            OperationSet::only(Operation::Observe),
        )
        .expect("returned resource and capability slots are reusable");
    assert_eq!(fresh.resource, ResourceId::new(4));
    assert!(fresh.resource.raw() > fixture.target.resource.raw());
    assert!(fresh.capability.raw() > fixture.target.capability.raw());
    assert_eq!(core.resources().len(), 3);
    assert_eq!(core.resources().last().unwrap().id, fresh.resource);
}

#[test]
fn cleanup_revocation_records_exact_rollback_evidence() {
    let (mut core, fixture) = setup::<32>(AgentEntryKind::Supervisor);
    core.retire_resource(
        fixture.actor,
        fixture.target.capability,
        fixture.target.resource,
    )
    .unwrap();
    let target = core.capability(fixture.target.capability).unwrap();
    let event_start = core.events().len();

    let event = core
        .revoke_capability_for_cleanup(fixture.actor, fixture.authority, fixture.target.capability)
        .expect("ancestor rollback authority revokes terminal capability");

    assert_eq!(event, core.events()[event_start]);
    assert_eq!(event.kind, EventKind::CapabilityRevoked);
    assert_eq!(event.agent, fixture.actor);
    assert_eq!(event.resource, Some(fixture.target.resource));
    assert_eq!(event.capability, Some(fixture.target.capability));
    assert_eq!(event.source_capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.operations, target.operations);
    assert_eq!(event.task, target.task);
    assert_eq!(event.target_agent, Some(target.agent));
    assert!(core.capability(fixture.target.capability).unwrap().revoked);
}

#[test]
fn child_and_capability_references_block_record_retirement() {
    let (mut child_core, fixture) = setup::<64>(AgentEntryKind::Supervisor);
    child_core
        .create_resource(
            fixture.actor,
            ResourceKind::Memory,
            Some((fixture.target.resource, fixture.target.capability)),
            OperationSet::only(Operation::Observe),
        )
        .unwrap();
    prepare_target(&mut child_core, fixture);
    assert_eq!(
        child_core.retire_resource_record(
            fixture.actor,
            fixture.authority,
            fixture.target.resource,
        ),
        Err(KernelError::ResourceRecordRetirementReferenced)
    );

    let (mut capability_core, fixture) = setup::<32>(AgentEntryKind::Supervisor);
    capability_core
        .retire_resource(
            fixture.actor,
            fixture.target.capability,
            fixture.target.resource,
        )
        .unwrap();
    assert_eq!(
        capability_core.retire_resource_record(
            fixture.actor,
            fixture.authority,
            fixture.target.resource,
        ),
        Err(KernelError::ResourceRecordRetirementReferenced)
    );
}
