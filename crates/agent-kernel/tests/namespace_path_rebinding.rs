use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, NamespaceKey, NamespaceObject, NamespacePathSegment, Operation,
    OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<1, 2, 2, 16, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

#[test]
fn namespace_path_rebind_syscall_forwards_atomic_receipt_and_events() {
    let mut kernel = TestKernel::new();
    let actor = AgentId::new(1);
    kernel.sys_register_agent(actor).unwrap();
    let root = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let child = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act);
    let root_authority = kernel.sys_grant(actor, root, operations).unwrap();
    let child_authority = kernel.sys_grant(actor, child, operations).unwrap();
    let root_key = NamespaceKey::new(0x1001);
    let terminal_key = NamespaceKey::new(0x1002);
    kernel
        .sys_bind_namespace_entry(
            actor,
            root_authority,
            root,
            root_key,
            NamespaceObject::Mount(child),
        )
        .unwrap();
    let terminal = kernel
        .sys_bind_namespace_entry(
            actor,
            child_authority,
            child,
            terminal_key,
            NamespaceObject::Agent(actor),
        )
        .unwrap();
    let event_start = kernel.events().len();

    let receipt = kernel
        .sys_compare_and_rebind_namespace_path(
            actor,
            root,
            &[
                NamespacePathSegment::new(root_authority, root_key),
                NamespacePathSegment::new(child_authority, terminal_key),
            ],
            1,
            NamespaceObject::Resource(root),
        )
        .unwrap();

    assert_eq!(receipt.root(), root);
    assert_eq!(receipt.depth(), 2);
    assert_eq!(receipt.previous().id, terminal);
    assert_eq!(receipt.previous().revision, 1);
    assert_eq!(receipt.rebound().id, terminal);
    assert_eq!(receipt.rebound().revision, 2);
    assert_eq!(receipt.rebound().object, NamespaceObject::Resource(root));
    assert_eq!(kernel.events().len(), event_start + 2);
    assert_eq!(
        kernel.events()[event_start].kind,
        EventKind::NamespaceEntryResolved
    );
    assert_eq!(
        kernel.events()[event_start + 1].kind,
        EventKind::NamespaceEntryRebound
    );
}
