use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, NamespaceEntryId, NamespaceKey, NamespaceObject, Operation, OperationSet,
    ResourceKind,
};

type TestKernel = AgentKernel<1, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn namespace_syscalls_bind_resolve_rebind_and_expose_entries() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
    kernel
        .sys_register_agent(agent)
        .expect("agent registration should fit");
    let workspace = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("workspace resource should fit");
    let capability = kernel
        .sys_grant(
            agent,
            workspace,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("workspace capability should fit");
    let key = NamespaceKey::new(11);

    let entry = kernel
        .sys_bind_namespace_entry(
            agent,
            capability,
            workspace,
            key,
            NamespaceObject::Resource(workspace),
        )
        .expect("namespace entry should fit");
    let resolved = kernel
        .sys_resolve_namespace_entry(agent, capability, workspace, key)
        .expect("namespace entry should resolve");
    let event = kernel
        .sys_rebind_namespace_entry(agent, capability, entry, NamespaceObject::Agent(agent))
        .expect("namespace entry should rebind");

    assert_eq!(entry, NamespaceEntryId::new(1));
    assert_eq!(resolved, NamespaceObject::Resource(workspace));
    assert_eq!(
        kernel.namespace_entries()[0].object,
        NamespaceObject::Agent(agent)
    );
    assert_eq!(kernel.namespace_entries()[0].revision, 2);
    assert_eq!(kernel.events()[2].kind, EventKind::NamespaceEntryBound);
    assert_eq!(kernel.events()[3].kind, EventKind::NamespaceEntryResolved);
    assert_eq!(event.kind, EventKind::NamespaceEntryRebound);
}
