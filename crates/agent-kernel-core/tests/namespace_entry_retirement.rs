mod namespace_entry_retirement_support;

use agent_kernel_core::{EventKind, NamespaceEntryId, NamespaceKey, NamespaceObject, Operation};

use namespace_entry_retirement_support::setup;

#[test]
fn retirement_preserves_dense_order_and_reuses_capacity_with_fresh_ids() {
    let (mut core, fixture) = setup::<32>();
    let removed = core.namespace_entries()[0];
    let event_start = core.events().len();

    let receipt = core
        .retire_namespace_entry(fixture.actor, fixture.authority, fixture.target)
        .expect("authorized entry retirement succeeds");

    assert_eq!(receipt.record(), removed);
    assert_eq!(receipt.namespace_entry(), fixture.target);
    assert_eq!(receipt.actor(), fixture.actor);
    assert_eq!(receipt.authority(), fixture.authority);
    assert_eq!(core.namespace_entries().len(), 1);
    assert_eq!(core.namespace_entries()[0].id, fixture.retained);

    let event = core.events()[event_start];
    assert_eq!(event.kind, EventKind::NamespaceEntryRetired);
    assert_eq!(event.agent, fixture.actor);
    assert_eq!(event.resource, Some(fixture.workspace));
    assert_eq!(event.capability, Some(fixture.authority));
    assert_eq!(event.namespace_entry, Some(fixture.target));
    assert_eq!(event.namespace_key, Some(NamespaceKey::new(11)));
    assert_eq!(
        event.namespace_object,
        Some(NamespaceObject::Resource(fixture.workspace))
    );
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.target_agent, Some(fixture.actor));

    let fresh = core
        .bind_namespace_entry(
            fixture.actor,
            fixture.authority,
            fixture.workspace,
            NamespaceKey::new(13),
            NamespaceObject::Resource(fixture.workspace),
        )
        .expect("returned slot is reusable");
    assert_eq!(fresh, NamespaceEntryId::new(3));
    assert!(fresh.raw() > fixture.target.raw());
    assert_eq!(
        core.namespace_entries()
            .iter()
            .map(|entry| entry.id.raw())
            .collect::<Vec<_>>(),
        [2, 3]
    );
}

#[test]
fn retirement_receipt_preserves_rebound_record() {
    let (mut core, fixture) = setup::<32>();
    core.rebind_namespace_entry(
        fixture.actor,
        fixture.authority,
        fixture.target,
        NamespaceObject::Agent(fixture.actor),
    )
    .unwrap();

    let receipt = core
        .retire_namespace_entry(fixture.actor, fixture.authority, fixture.target)
        .unwrap();

    assert_eq!(receipt.record().owner, fixture.actor);
    assert_eq!(receipt.record().namespace, fixture.workspace);
    assert_eq!(receipt.record().capability, fixture.authority);
    assert_eq!(receipt.record().key, NamespaceKey::new(11));
    assert_eq!(
        receipt.record().object,
        NamespaceObject::Agent(fixture.actor)
    );
    assert_eq!(receipt.record().revision, 2);
}
