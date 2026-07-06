use agent_kernel_core::{
    AgentId, KernelCore, KernelError, MemoryCellId, NamespaceEntryId, NamespaceKey,
    NamespaceObject, Operation, OperationSet, ResourceKind,
};

type TestCore = KernelCore<2, 3, 4, 16, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

fn setup_namespace_core() -> (
    TestCore,
    AgentId,
    agent_kernel_core::ResourceId,
    agent_kernel_core::CapabilityId,
) {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    core.register_agent(agent)
        .expect("agent registration should fit");
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("workspace resource should fit");
    let capability = core
        .grant_capability(
            agent,
            workspace,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("workspace capability should fit");

    (core, agent, workspace, capability)
}

#[test]
fn bind_namespace_entry_rejects_non_workspace_resource_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    core.register_agent(agent)
        .expect("agent registration should fit");
    let memory = core
        .register_resource(ResourceKind::Memory, None)
        .expect("memory resource should fit");
    let capability = core
        .grant_capability(agent, memory, OperationSet::only(Operation::Act))
        .expect("memory capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.bind_namespace_entry(
            agent,
            capability,
            memory,
            NamespaceKey::new(1),
            NamespaceObject::Resource(memory),
        ),
        Err(KernelError::ResourceKindMismatch)
    );
    assert!(core.namespace_entries().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn bind_namespace_entry_rejects_duplicate_key_without_event() {
    let (mut core, agent, workspace, capability) = setup_namespace_core();
    let key = NamespaceKey::new(1);
    core.bind_namespace_entry(
        agent,
        capability,
        workspace,
        key,
        NamespaceObject::Resource(workspace),
    )
    .expect("first namespace entry should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.bind_namespace_entry(
            agent,
            capability,
            workspace,
            key,
            NamespaceObject::Agent(agent),
        ),
        Err(KernelError::NamespaceEntryAlreadyExists)
    );
    assert_eq!(core.namespace_entries().len(), 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn resolve_namespace_entry_reports_missing_binding_without_event() {
    let (mut core, agent, workspace, capability) = setup_namespace_core();
    let events_before = core.events().len();

    assert_eq!(
        core.resolve_namespace_entry(agent, capability, workspace, NamespaceKey::new(99)),
        Err(KernelError::NamespaceEntryNotFound)
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn bind_namespace_entry_rejects_missing_referenced_object_without_event() {
    let (mut core, agent, workspace, capability) = setup_namespace_core();
    let events_before = core.events().len();

    assert_eq!(
        core.bind_namespace_entry(
            agent,
            capability,
            workspace,
            NamespaceKey::new(2),
            NamespaceObject::MemoryCell(MemoryCellId::new(99)),
        ),
        Err(KernelError::MemoryCellNotFound)
    );
    assert!(core.namespace_entries().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn rebind_namespace_entry_requires_act_authority_without_mutation() {
    let (mut core, agent, workspace, capability) = setup_namespace_core();
    let entry = core
        .bind_namespace_entry(
            agent,
            capability,
            workspace,
            NamespaceKey::new(3),
            NamespaceObject::Resource(workspace),
        )
        .expect("namespace entry should fit");
    let observe_only = core
        .grant_capability(agent, workspace, OperationSet::only(Operation::Observe))
        .expect("observe capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.rebind_namespace_entry(agent, observe_only, entry, NamespaceObject::Agent(agent)),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(
        core.namespace_entries()[0].object,
        NamespaceObject::Resource(workspace)
    );
    assert_eq!(core.namespace_entries()[0].revision, 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn suspended_actor_is_rejected_before_namespace_lookup() {
    let (mut core, agent, _, _) = setup_namespace_core();
    core.suspend_agent(agent).expect("agent should suspend");
    let events_before = core.events().len();

    assert_eq!(
        core.rebind_namespace_entry(
            agent,
            agent_kernel_core::CapabilityId::new(99),
            NamespaceEntryId::new(99),
            NamespaceObject::Agent(agent),
        ),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(core.events().len(), events_before);
}
