use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, NamespaceKey, NamespaceObject, NamespacePathSegment, Operation,
    OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<1, 2, 2, 16, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

#[test]
fn namespace_path_syscall_resolves_mounts_and_exposes_terminal_record() {
    let mut kernel = TestKernel::new();
    let actor = AgentId::new(1);
    kernel
        .sys_register_agent(actor)
        .expect("agent registration should fit");
    let root = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("root workspace should fit");
    let child = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("child workspace should fit");
    let root_authority = kernel
        .sys_grant(
            actor,
            root,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("root authority should fit");
    let child_authority = kernel
        .sys_grant(
            actor,
            child,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("child authority should fit");
    let root_key = NamespaceKey::new(0x4e53_0001);
    let child_key = NamespaceKey::new(0x4e53_0002);

    kernel
        .sys_bind_namespace_entry(
            actor,
            root_authority,
            root,
            root_key,
            NamespaceObject::Mount(child),
        )
        .expect("mount entry should bind");
    let terminal = kernel
        .sys_bind_namespace_entry(
            actor,
            child_authority,
            child,
            child_key,
            NamespaceObject::Agent(actor),
        )
        .expect("terminal entry should bind");
    let before = kernel.events().len();

    let resolution = kernel
        .sys_resolve_namespace_path(
            actor,
            root,
            &[
                NamespacePathSegment::new(root_authority, root_key),
                NamespacePathSegment::new(child_authority, child_key),
            ],
        )
        .expect("two-hop path should resolve");

    assert_eq!(resolution.root(), root);
    assert_eq!(resolution.depth(), 2);
    assert_eq!(resolution.terminal().id, terminal);
    assert_eq!(resolution.terminal().namespace, child);
    assert_eq!(resolution.terminal().object, NamespaceObject::Agent(actor));
    assert_eq!(kernel.events().len(), before + 2);
    assert_eq!(
        kernel.events()[before].kind,
        EventKind::NamespaceEntryResolved
    );
    assert_eq!(
        kernel.events()[before].namespace_entry,
        kernel.namespace_entries().first().map(|entry| entry.id)
    );
    assert_eq!(
        kernel.events()[before + 1].kind,
        EventKind::NamespaceEntryResolved
    );
    assert_eq!(kernel.events()[before + 1].namespace_entry, Some(terminal));
}
