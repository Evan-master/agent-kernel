use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, MemoryCellId, NamespaceKey, NamespaceObject,
    NamespacePathSegment, Operation, OperationSet, ResourceId, ResourceKind,
    NAMESPACE_PATH_MAX_DEPTH,
};

type TestCore = KernelCore<2, 4, 8, 32, 0, 0, 0, 0, 0, 0, 0, 0, 8>;

#[derive(Copy, Clone)]
struct Fixture {
    actor: AgentId,
    root: ResourceId,
    root_authority: agent_kernel_core::CapabilityId,
    child_authority: agent_kernel_core::CapabilityId,
    leaf_authority: agent_kernel_core::CapabilityId,
    root_key: NamespaceKey,
    child_key: NamespaceKey,
    terminal_key: NamespaceKey,
}

impl Fixture {
    fn path(self) -> [NamespacePathSegment; 3] {
        [
            NamespacePathSegment::new(self.root_authority, self.root_key),
            NamespacePathSegment::new(self.child_authority, self.child_key),
            NamespacePathSegment::new(self.leaf_authority, self.terminal_key),
        ]
    }
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
    let leaf = core
        .register_resource(ResourceKind::Workspace, Some(child))
        .unwrap();
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback);
    let root_authority = core.grant_capability(actor, root, operations).unwrap();
    let child_authority = core.grant_capability(actor, child, operations).unwrap();
    let leaf_authority = core.grant_capability(actor, leaf, operations).unwrap();
    let root_key = NamespaceKey::new(0x1001);
    let child_key = NamespaceKey::new(0x1002);
    let terminal_key = NamespaceKey::new(0x1003);
    core.bind_namespace_entry(
        actor,
        root_authority,
        root,
        root_key,
        NamespaceObject::Mount(child),
    )
    .unwrap();
    core.bind_namespace_entry(
        actor,
        child_authority,
        child,
        child_key,
        NamespaceObject::Mount(leaf),
    )
    .unwrap();
    core.bind_namespace_entry(
        actor,
        leaf_authority,
        leaf,
        terminal_key,
        NamespaceObject::Agent(actor),
    )
    .unwrap();

    (
        core,
        Fixture {
            actor,
            root,
            root_authority,
            child_authority,
            leaf_authority,
            root_key,
            child_key,
            terminal_key,
        },
    )
}

#[test]
fn path_rebind_resolves_mounts_and_commits_one_terminal_revision() {
    let (mut core, fixture) = setup();
    let previous = core.namespace_entries()[2];
    let event_start = core.events().len();

    let receipt = core
        .compare_and_rebind_namespace_path(
            fixture.actor,
            fixture.root,
            &fixture.path(),
            1,
            NamespaceObject::Resource(fixture.root),
        )
        .unwrap();

    assert_eq!(receipt.root(), fixture.root);
    assert_eq!(receipt.depth(), 3);
    assert_eq!(receipt.previous(), previous);
    assert_eq!(receipt.rebound().id, previous.id);
    assert_eq!(receipt.rebound().revision, 2);
    assert_eq!(
        receipt.rebound().object,
        NamespaceObject::Resource(fixture.root)
    );
    assert_eq!(core.namespace_entries().len(), 3);
    assert_eq!(core.namespace_entries()[2], receipt.rebound());
    assert_eq!(core.events().len(), event_start + 3);
    assert_eq!(
        core.events()[event_start].kind,
        EventKind::NamespaceEntryResolved
    );
    assert_eq!(
        core.events()[event_start].capability,
        Some(fixture.root_authority)
    );
    assert_eq!(
        core.events()[event_start + 1].kind,
        EventKind::NamespaceEntryResolved
    );
    assert_eq!(
        core.events()[event_start + 1].capability,
        Some(fixture.child_authority)
    );
    assert_eq!(
        core.events()[event_start + 2].kind,
        EventKind::NamespaceEntryRebound
    );
    assert_eq!(
        core.events()[event_start + 2].capability,
        Some(fixture.leaf_authority)
    );
    assert_eq!(
        core.events()[event_start + 2].namespace_object,
        Some(NamespaceObject::Resource(fixture.root))
    );
}

#[test]
fn terminal_authority_and_revision_fail_before_events_or_mutation() {
    let (mut core, fixture) = setup();
    let records = [
        core.namespace_entries()[0],
        core.namespace_entries()[1],
        core.namespace_entries()[2],
    ];
    let event_start = core.events().len();
    let mut wrong_terminal = fixture.path();
    wrong_terminal[2] = NamespacePathSegment::new(fixture.root_authority, fixture.terminal_key);

    assert_eq!(
        core.compare_and_rebind_namespace_path(
            fixture.actor,
            fixture.root,
            &wrong_terminal,
            99,
            NamespaceObject::Resource(fixture.root),
        ),
        Err(KernelError::ResourceMismatch)
    );
    assert_eq!(
        core.compare_and_rebind_namespace_path(
            fixture.actor,
            fixture.root,
            &fixture.path(),
            99,
            NamespaceObject::Resource(fixture.root),
        ),
        Err(KernelError::NamespaceRevisionMismatch)
    );
    assert_eq!(core.namespace_entries(), &records);
    assert_eq!(core.events().len(), event_start);
}

#[test]
fn invalid_replacement_and_mount_cycle_are_atomic() {
    let (mut core, fixture) = setup();
    let records = [
        core.namespace_entries()[0],
        core.namespace_entries()[1],
        core.namespace_entries()[2],
    ];
    let event_start = core.events().len();

    assert_eq!(
        core.compare_and_rebind_namespace_path(
            fixture.actor,
            fixture.root,
            &fixture.path(),
            1,
            NamespaceObject::MemoryCell(MemoryCellId::new(99)),
        ),
        Err(KernelError::MemoryCellNotFound)
    );
    assert_eq!(
        core.compare_and_rebind_namespace_path(
            fixture.actor,
            fixture.root,
            &fixture.path(),
            1,
            NamespaceObject::Mount(fixture.root),
        ),
        Err(KernelError::NamespaceMountCycle)
    );
    assert_eq!(core.namespace_entries(), &records);
    assert_eq!(core.events().len(), event_start);
}

#[test]
fn path_shape_and_event_capacity_failures_preserve_terminal_record() {
    let (mut core, fixture) = setup();
    let event_start = core.events().len();
    assert_eq!(
        core.compare_and_rebind_namespace_path(
            fixture.actor,
            fixture.root,
            &[],
            1,
            NamespaceObject::Resource(fixture.root),
        ),
        Err(KernelError::NamespacePathEmpty)
    );
    let oversized = [NamespacePathSegment::new(fixture.root_authority, fixture.root_key);
        NAMESPACE_PATH_MAX_DEPTH + 1];
    assert_eq!(
        core.compare_and_rebind_namespace_path(
            fixture.actor,
            fixture.root,
            &oversized,
            1,
            NamespaceObject::Resource(fixture.root),
        ),
        Err(KernelError::NamespacePathTooDeep)
    );
    assert_eq!(core.events().len(), event_start);

    let mut full = KernelCore::<1, 3, 3, 9, 0, 0, 0, 0, 0, 0, 0, 0, 3>::new();
    let actor = AgentId::new(1);
    full.register_agent(actor).unwrap();
    let root = full
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let child = full
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let leaf = full
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act);
    let root_authority = full.grant_capability(actor, root, operations).unwrap();
    let child_authority = full.grant_capability(actor, child, operations).unwrap();
    let leaf_authority = full.grant_capability(actor, leaf, operations).unwrap();
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
        NamespaceObject::Mount(leaf),
    )
    .unwrap();
    full.bind_namespace_entry(
        actor,
        leaf_authority,
        leaf,
        NamespaceKey::new(3),
        NamespaceObject::Agent(actor),
    )
    .unwrap();
    let records = [
        full.namespace_entries()[0],
        full.namespace_entries()[1],
        full.namespace_entries()[2],
    ];
    let event_count = full.events().len();

    assert_eq!(
        full.compare_and_rebind_namespace_path(
            actor,
            root,
            &[
                NamespacePathSegment::new(root_authority, NamespaceKey::new(1)),
                NamespacePathSegment::new(child_authority, NamespaceKey::new(2)),
                NamespacePathSegment::new(leaf_authority, NamespaceKey::new(3)),
            ],
            1,
            NamespaceObject::Resource(root),
        ),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(full.namespace_entries(), &records);
    assert_eq!(full.events().len(), event_count);
}
