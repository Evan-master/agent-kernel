use agent_kernel_core::{
    AgentId, KernelCore, KernelError, NamespaceKey, NamespaceObject, Operation, OperationSet,
    ResourceKind,
};

#[test]
fn bind_namespace_entry_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should fit");
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("workspace should fit");
    let capability = core
        .grant_capability(agent, workspace, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    core.bind_namespace_entry(
        agent,
        capability,
        workspace,
        NamespaceKey::new(1),
        NamespaceObject::Resource(workspace),
    )
    .expect("first entry should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.bind_namespace_entry(
            agent,
            capability,
            workspace,
            NamespaceKey::new(2),
            NamespaceObject::Agent(agent),
        ),
        Err(KernelError::NamespaceEntryStoreFull)
    );
    assert_eq!(core.namespace_entries().len(), 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn bind_namespace_entry_event_log_full_leaves_entries_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent)
        .expect("registration should consume one event");
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("workspace should fit");
    let capability = core
        .grant_capability(agent, workspace, OperationSet::only(Operation::Act))
        .expect("grant should consume final event");

    assert_eq!(
        core.bind_namespace_entry(
            agent,
            capability,
            workspace,
            NamespaceKey::new(1),
            NamespaceObject::Resource(workspace),
        ),
        Err(KernelError::EventLogFull)
    );
    assert!(core.namespace_entries().is_empty());
    assert_eq!(core.events().len(), 2);
}

#[test]
fn resolve_namespace_entry_event_log_full_does_not_return_unaudited_object() {
    let mut core = KernelCore::<1, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should fit");
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("workspace should fit");
    let capability = core
        .grant_capability(
            agent,
            workspace,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("capability should fit");
    core.bind_namespace_entry(
        agent,
        capability,
        workspace,
        NamespaceKey::new(1),
        NamespaceObject::Resource(workspace),
    )
    .expect("bind should consume final event");

    assert_eq!(
        core.resolve_namespace_entry(agent, capability, workspace, NamespaceKey::new(1)),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.namespace_entries()[0].revision, 1);
    assert_eq!(core.events().len(), 3);
}

#[test]
fn rebind_namespace_entry_event_log_full_leaves_object_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 1>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should fit");
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("workspace should fit");
    let capability = core
        .grant_capability(agent, workspace, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let entry = core
        .bind_namespace_entry(
            agent,
            capability,
            workspace,
            NamespaceKey::new(1),
            NamespaceObject::Resource(workspace),
        )
        .expect("bind should consume final event");

    assert_eq!(
        core.rebind_namespace_entry(agent, capability, entry, NamespaceObject::Agent(agent)),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(
        core.namespace_entries()[0].object,
        NamespaceObject::Resource(workspace)
    );
    assert_eq!(core.namespace_entries()[0].revision, 1);
    assert_eq!(core.events().len(), 3);
}
