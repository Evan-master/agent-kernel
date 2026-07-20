use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, NamespaceKey, NamespaceObject, Operation, OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<1, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn namespace_generation_syscalls_return_complete_records_and_receipts() {
    let mut kernel = TestKernel::new();
    let actor = AgentId::new(1);
    kernel.sys_register_agent(actor).unwrap();
    let workspace = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = kernel
        .sys_grant(
            actor,
            workspace,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .unwrap();
    let entry = kernel
        .sys_bind_namespace_entry(
            actor,
            authority,
            workspace,
            NamespaceKey::new(7),
            NamespaceObject::Resource(workspace),
        )
        .unwrap();

    let rebound = kernel
        .sys_compare_and_rebind_namespace_entry(
            actor,
            authority,
            entry,
            1,
            NamespaceObject::Agent(actor),
        )
        .unwrap();
    let retired = kernel
        .sys_compare_and_retire_namespace_entry(actor, authority, entry, 2)
        .unwrap();

    assert_eq!(rebound.id, entry);
    assert_eq!(rebound.revision, 2);
    assert_eq!(retired.record(), rebound);
    assert!(kernel.namespace_entries().is_empty());
}
