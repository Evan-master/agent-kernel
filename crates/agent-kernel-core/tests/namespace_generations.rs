use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, MemoryCellId, NamespaceEntryId, NamespaceKey,
    NamespaceObject, Operation, OperationSet, ResourceId, ResourceKind,
};

type TestCore = KernelCore<2, 2, 4, 16, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

struct Fixture {
    actor: AgentId,
    workspace: ResourceId,
    authority: agent_kernel_core::CapabilityId,
    entry: NamespaceEntryId,
}

fn setup() -> (TestCore, Fixture) {
    let mut core = TestCore::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).unwrap();
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = core
        .grant_capability(
            actor,
            workspace,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .unwrap();
    let entry = core
        .bind_namespace_entry(
            actor,
            authority,
            workspace,
            NamespaceKey::new(0x100),
            NamespaceObject::Resource(workspace),
        )
        .unwrap();

    (
        core,
        Fixture {
            actor,
            workspace,
            authority,
            entry,
        },
    )
}

#[test]
fn compare_rebind_and_retire_advance_exact_generation_and_reuse_capacity() {
    let (mut core, fixture) = setup();

    let rebound = core
        .compare_and_rebind_namespace_entry(
            fixture.actor,
            fixture.authority,
            fixture.entry,
            1,
            NamespaceObject::Agent(fixture.actor),
        )
        .unwrap();
    let retired = core
        .compare_and_retire_namespace_entry(fixture.actor, fixture.authority, fixture.entry, 2)
        .unwrap();
    let fresh = core
        .bind_namespace_entry(
            fixture.actor,
            fixture.authority,
            fixture.workspace,
            NamespaceKey::new(0x101),
            NamespaceObject::Resource(fixture.workspace),
        )
        .unwrap();

    assert_eq!(rebound.id, fixture.entry);
    assert_eq!(rebound.object, NamespaceObject::Agent(fixture.actor));
    assert_eq!(rebound.revision, 2);
    assert_eq!(retired.record(), rebound);
    assert_eq!(fresh, NamespaceEntryId::new(2));
    assert_eq!(core.namespace_entries().len(), 1);
    assert_eq!(core.namespace_entries()[0].id, fresh);
    assert_eq!(core.events()[3].kind, EventKind::NamespaceEntryRebound);
    assert_eq!(core.events()[4].kind, EventKind::NamespaceEntryRetired);
    assert_eq!(core.events()[5].kind, EventKind::NamespaceEntryBound);
}

#[test]
fn stale_compare_operations_preserve_record_revision_events_and_capacity() {
    let (mut core, fixture) = setup();
    let current = core
        .compare_and_rebind_namespace_entry(
            fixture.actor,
            fixture.authority,
            fixture.entry,
            1,
            NamespaceObject::Agent(fixture.actor),
        )
        .unwrap();
    let events_before = core.events().len();

    assert_eq!(
        core.compare_and_rebind_namespace_entry(
            fixture.actor,
            fixture.authority,
            fixture.entry,
            1,
            NamespaceObject::Resource(fixture.workspace),
        ),
        Err(KernelError::NamespaceRevisionMismatch)
    );
    assert_eq!(
        core.compare_and_retire_namespace_entry(
            fixture.actor,
            fixture.authority,
            fixture.entry,
            1,
        ),
        Err(KernelError::NamespaceRevisionMismatch)
    );
    assert_eq!(core.namespace_entries(), &[current]);
    assert_eq!(core.namespace_entry_capacity(), 2);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn authorization_precedes_revision_comparison_without_mutation() {
    let (mut core, fixture) = setup();
    let observe = core
        .grant_capability(
            fixture.actor,
            fixture.workspace,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();
    let record = core.namespace_entries()[0];
    let events_before = core.events().len();

    assert_eq!(
        core.compare_and_rebind_namespace_entry(
            fixture.actor,
            observe,
            fixture.entry,
            99,
            NamespaceObject::Agent(fixture.actor),
        ),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(
        core.compare_and_retire_namespace_entry(fixture.actor, observe, fixture.entry, 99,),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.namespace_entries(), &[record]);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn compare_rebind_validates_object_after_revision_without_mutation() {
    let (mut core, fixture) = setup();
    let record = core.namespace_entries()[0];
    let events_before = core.events().len();

    assert_eq!(
        core.compare_and_rebind_namespace_entry(
            fixture.actor,
            fixture.authority,
            fixture.entry,
            1,
            NamespaceObject::MemoryCell(MemoryCellId::new(99)),
        ),
        Err(KernelError::MemoryCellNotFound)
    );
    assert_eq!(core.namespace_entries(), &[record]);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn compare_operations_are_atomic_when_event_log_is_full() {
    let mut core = KernelCore::<1, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).unwrap();
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = core
        .grant_capability(
            actor,
            workspace,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .unwrap();
    let entry = core
        .bind_namespace_entry(
            actor,
            authority,
            workspace,
            NamespaceKey::new(1),
            NamespaceObject::Resource(workspace),
        )
        .unwrap();
    let record = core.namespace_entries()[0];

    assert_eq!(
        core.compare_and_rebind_namespace_entry(
            actor,
            authority,
            entry,
            1,
            NamespaceObject::Agent(actor),
        ),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(
        core.compare_and_retire_namespace_entry(actor, authority, entry, 1),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.namespace_entries(), &[record]);
    assert_eq!(core.events().len(), 3);
}
