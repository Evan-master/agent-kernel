use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, NamespaceKey, NamespaceObject,
    NamespacePathSegment, Operation, OperationSet, ResourceId, ResourceKind,
    NAMESPACE_PATH_MAX_DEPTH,
};

type TestCore = KernelCore<2, 4, 8, 32, 0, 0, 0, 0, 0, 0, 0, 0, 8>;

#[derive(Copy, Clone)]
struct Fixture {
    actor: AgentId,
    root: ResourceId,
    child: ResourceId,
    root_authority: agent_kernel_core::CapabilityId,
    child_authority: agent_kernel_core::CapabilityId,
}

fn setup() -> (TestCore, Fixture) {
    let mut core = TestCore::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).unwrap();
    let root = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let child = core
        .register_resource(ResourceKind::Workspace, Some(root))
        .unwrap();
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);
    let root_authority = core.grant_capability(actor, root, operations).unwrap();
    let child_authority = core.grant_capability(actor, child, operations).unwrap();
    (
        core,
        Fixture {
            actor,
            root,
            child,
            root_authority,
            child_authority,
        },
    )
}

#[test]
fn two_hop_resolution_uses_independent_authority_and_records_each_hop() {
    let (mut core, fixture) = setup();
    let mount_key = NamespaceKey::new(0x100);
    let terminal_key = NamespaceKey::new(0x200);
    let mount = core
        .bind_namespace_entry(
            fixture.actor,
            fixture.root_authority,
            fixture.root,
            mount_key,
            NamespaceObject::Mount(fixture.child),
        )
        .unwrap();
    let terminal = core
        .bind_namespace_entry(
            fixture.actor,
            fixture.child_authority,
            fixture.child,
            terminal_key,
            NamespaceObject::Agent(fixture.actor),
        )
        .unwrap();
    let event_start = core.events().len();

    let resolution = core
        .resolve_namespace_path(
            fixture.actor,
            fixture.root,
            &[
                NamespacePathSegment::new(fixture.root_authority, mount_key),
                NamespacePathSegment::new(fixture.child_authority, terminal_key),
            ],
        )
        .unwrap();

    assert_eq!(resolution.root(), fixture.root);
    assert_eq!(resolution.depth(), 2);
    assert_eq!(resolution.terminal().id, terminal);
    assert_eq!(resolution.terminal().namespace, fixture.child);
    assert_eq!(
        resolution.terminal().object,
        NamespaceObject::Agent(fixture.actor)
    );
    assert_eq!(core.events().len(), event_start + 2);
    assert_eq!(
        core.events()[event_start].kind,
        EventKind::NamespaceEntryResolved
    );
    assert_eq!(core.events()[event_start].namespace_entry, Some(mount));
    assert_eq!(
        core.events()[event_start].capability,
        Some(fixture.root_authority)
    );
    assert_eq!(
        core.events()[event_start + 1].kind,
        EventKind::NamespaceEntryResolved
    );
    assert_eq!(
        core.events()[event_start + 1].namespace_entry,
        Some(terminal)
    );
    assert_eq!(
        core.events()[event_start + 1].capability,
        Some(fixture.child_authority)
    );
}

#[test]
fn late_hop_authority_and_non_mount_fail_before_any_event() {
    let (mut core, fixture) = setup();
    let mount_key = NamespaceKey::new(0x100);
    let terminal_key = NamespaceKey::new(0x200);
    core.bind_namespace_entry(
        fixture.actor,
        fixture.root_authority,
        fixture.root,
        mount_key,
        NamespaceObject::Mount(fixture.child),
    )
    .unwrap();
    core.bind_namespace_entry(
        fixture.actor,
        fixture.child_authority,
        fixture.child,
        terminal_key,
        NamespaceObject::Agent(fixture.actor),
    )
    .unwrap();
    let event_start = core.events().len();

    assert_eq!(
        core.resolve_namespace_path(
            fixture.actor,
            fixture.root,
            &[
                NamespacePathSegment::new(fixture.root_authority, mount_key),
                NamespacePathSegment::new(fixture.root_authority, terminal_key),
            ],
        ),
        Err(KernelError::ResourceMismatch)
    );
    assert_eq!(core.events().len(), event_start);

    let opaque_key = NamespaceKey::new(0x300);
    core.bind_namespace_entry(
        fixture.actor,
        fixture.root_authority,
        fixture.root,
        opaque_key,
        NamespaceObject::Resource(fixture.child),
    )
    .unwrap();
    let event_start = core.events().len();
    assert_eq!(
        core.resolve_namespace_path(
            fixture.actor,
            fixture.root,
            &[
                NamespacePathSegment::new(fixture.root_authority, opaque_key),
                NamespacePathSegment::new(fixture.child_authority, terminal_key),
            ],
        ),
        Err(KernelError::NamespaceMountRequired)
    );
    assert_eq!(core.events().len(), event_start);
}

#[test]
fn mount_binding_rejects_kind_retirement_and_direct_cycle_atomically() {
    let (mut core, fixture) = setup();
    let memory = core
        .register_resource(ResourceKind::Memory, Some(fixture.root))
        .unwrap();
    let events_before = core.events().len();

    assert_eq!(
        core.bind_namespace_entry(
            fixture.actor,
            fixture.root_authority,
            fixture.root,
            NamespaceKey::new(1),
            NamespaceObject::Mount(memory),
        ),
        Err(KernelError::ResourceKindMismatch)
    );
    assert_eq!(
        core.bind_namespace_entry(
            fixture.actor,
            fixture.root_authority,
            fixture.root,
            NamespaceKey::new(2),
            NamespaceObject::Mount(fixture.root),
        ),
        Err(KernelError::NamespaceMountCycle)
    );
    assert_eq!(core.events().len(), events_before);
    assert!(core.namespace_entries().is_empty());

    core.retire_resource(fixture.actor, fixture.child_authority, fixture.child)
        .unwrap();
    let events_before = core.events().len();
    assert_eq!(
        core.bind_namespace_entry(
            fixture.actor,
            fixture.root_authority,
            fixture.root,
            NamespaceKey::new(3),
            NamespaceObject::Mount(fixture.child),
        ),
        Err(KernelError::ResourceRetired)
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn transitive_cycle_is_rejected_for_bind_force_rebind_and_compare_rebind() {
    let (mut core, fixture) = setup();
    let leaf = core
        .register_resource(ResourceKind::Workspace, Some(fixture.child))
        .unwrap();
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);
    let leaf_authority = core
        .grant_capability(fixture.actor, leaf, operations)
        .unwrap();
    core.bind_namespace_entry(
        fixture.actor,
        fixture.root_authority,
        fixture.root,
        NamespaceKey::new(1),
        NamespaceObject::Mount(fixture.child),
    )
    .unwrap();
    core.bind_namespace_entry(
        fixture.actor,
        fixture.child_authority,
        fixture.child,
        NamespaceKey::new(2),
        NamespaceObject::Mount(leaf),
    )
    .unwrap();
    let terminal = core
        .bind_namespace_entry(
            fixture.actor,
            leaf_authority,
            leaf,
            NamespaceKey::new(3),
            NamespaceObject::Agent(fixture.actor),
        )
        .unwrap();
    let terminal_record = core
        .namespace_entries()
        .iter()
        .find(|record| record.id == terminal)
        .copied()
        .unwrap();
    let events_before = core.events().len();

    assert_eq!(
        core.bind_namespace_entry(
            fixture.actor,
            leaf_authority,
            leaf,
            NamespaceKey::new(4),
            NamespaceObject::Mount(fixture.root),
        ),
        Err(KernelError::NamespaceMountCycle)
    );
    assert_eq!(
        core.rebind_namespace_entry(
            fixture.actor,
            leaf_authority,
            terminal,
            NamespaceObject::Mount(fixture.root),
        ),
        Err(KernelError::NamespaceMountCycle)
    );
    assert_eq!(
        core.compare_and_rebind_namespace_entry(
            fixture.actor,
            leaf_authority,
            terminal,
            1,
            NamespaceObject::Mount(fixture.root),
        ),
        Err(KernelError::NamespaceMountCycle)
    );
    assert_eq!(core.events().len(), events_before);
    assert_eq!(
        core.namespace_entries()
            .iter()
            .find(|record| record.id == terminal)
            .copied(),
        Some(terminal_record)
    );
}

#[test]
fn path_shape_and_event_capacity_failures_are_atomic() {
    let (mut core, fixture) = setup();
    let events_before = core.events().len();
    assert_eq!(
        core.resolve_namespace_path(fixture.actor, fixture.root, &[]),
        Err(KernelError::NamespacePathEmpty)
    );
    let oversized = [NamespacePathSegment::new(fixture.root_authority, NamespaceKey::new(1));
        NAMESPACE_PATH_MAX_DEPTH + 1];
    assert_eq!(
        core.resolve_namespace_path(fixture.actor, fixture.root, &oversized),
        Err(KernelError::NamespacePathTooDeep)
    );
    assert_eq!(core.events().len(), events_before);

    let mut full = KernelCore::<1, 2, 2, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2>::new();
    let actor = AgentId::new(1);
    full.register_agent(actor).unwrap();
    let root = full
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let child = full
        .register_resource(ResourceKind::Workspace, Some(root))
        .unwrap();
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act);
    let root_authority = full.grant_capability(actor, root, operations).unwrap();
    let child_authority = full.grant_capability(actor, child, operations).unwrap();
    full.bind_namespace_entry(
        actor,
        root_authority,
        root,
        NamespaceKey::new(1),
        NamespaceObject::Mount(child),
    )
    .unwrap();
    full.bind_namespace_entry(
        actor,
        child_authority,
        child,
        NamespaceKey::new(2),
        NamespaceObject::Agent(actor),
    )
    .unwrap();
    let records = [full.namespace_entries()[0], full.namespace_entries()[1]];

    assert_eq!(
        full.resolve_namespace_path(
            actor,
            root,
            &[
                NamespacePathSegment::new(root_authority, NamespaceKey::new(1)),
                NamespacePathSegment::new(child_authority, NamespaceKey::new(2)),
            ],
        ),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(full.namespace_entries(), &records);
    assert_eq!(full.events().len(), 5);
}
