use agent_kernel_core::{
    AgentId, EventKind, KernelCore, NamespaceEntryId, NamespaceKey, NamespaceObject, Operation,
    OperationSet, ResourceId, ResourceKind,
};

type TestCore = KernelCore<2, 2, 2, 16, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

fn setup_namespace_core() -> (
    TestCore,
    AgentId,
    ResourceId,
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
fn bind_namespace_entry_records_object_and_event() {
    let (mut core, agent, workspace, capability) = setup_namespace_core();
    let key = NamespaceKey::new(7);
    let object = NamespaceObject::Resource(workspace);

    let entry = core
        .bind_namespace_entry(agent, capability, workspace, key, object)
        .expect("namespace entry should fit");

    assert_eq!(entry, NamespaceEntryId::new(1));
    assert_eq!(core.namespace_entries().len(), 1);
    assert_eq!(core.namespace_entries()[0].id, entry);
    assert_eq!(core.namespace_entries()[0].owner, agent);
    assert_eq!(core.namespace_entries()[0].namespace, workspace);
    assert_eq!(core.namespace_entries()[0].capability, capability);
    assert_eq!(core.namespace_entries()[0].key, key);
    assert_eq!(core.namespace_entries()[0].object, object);
    assert_eq!(core.namespace_entries()[0].revision, 1);
    assert_eq!(core.events()[2].kind, EventKind::NamespaceEntryBound);
    assert_eq!(core.events()[2].agent, agent);
    assert_eq!(core.events()[2].resource, Some(workspace));
    assert_eq!(core.events()[2].capability, Some(capability));
    assert_eq!(core.events()[2].namespace_entry, Some(entry));
    assert_eq!(core.events()[2].namespace_key, Some(key));
    assert_eq!(core.events()[2].namespace_object, Some(object));
    assert_eq!(core.events()[2].operation, Some(Operation::Act));
}

#[test]
fn resolve_namespace_entry_returns_object_and_records_audit_event() {
    let (mut core, agent, workspace, capability) = setup_namespace_core();
    let key = NamespaceKey::new(8);
    let object = NamespaceObject::Resource(workspace);
    let entry = core
        .bind_namespace_entry(agent, capability, workspace, key, object)
        .expect("namespace entry should fit");

    let resolved = core
        .resolve_namespace_entry(agent, capability, workspace, key)
        .expect("namespace entry should resolve");

    assert_eq!(resolved, object);
    assert_eq!(core.events()[3].kind, EventKind::NamespaceEntryResolved);
    assert_eq!(core.events()[3].agent, agent);
    assert_eq!(core.events()[3].resource, Some(workspace));
    assert_eq!(core.events()[3].namespace_entry, Some(entry));
    assert_eq!(core.events()[3].namespace_key, Some(key));
    assert_eq!(core.events()[3].namespace_object, Some(object));
    assert_eq!(core.events()[3].operation, Some(Operation::Observe));
}

#[test]
fn rebind_namespace_entry_updates_object_revision_and_event() {
    let (mut core, agent, workspace, capability) = setup_namespace_core();
    let key = NamespaceKey::new(9);
    let entry = core
        .bind_namespace_entry(
            agent,
            capability,
            workspace,
            key,
            NamespaceObject::Resource(workspace),
        )
        .expect("namespace entry should fit");
    let new_object = NamespaceObject::Agent(agent);

    let event = core
        .rebind_namespace_entry(agent, capability, entry, new_object)
        .expect("namespace entry should rebind");

    assert_eq!(core.namespace_entries()[0].object, new_object);
    assert_eq!(core.namespace_entries()[0].revision, 2);
    assert_eq!(event.kind, EventKind::NamespaceEntryRebound);
    assert_eq!(event.agent, agent);
    assert_eq!(event.resource, Some(workspace));
    assert_eq!(event.namespace_entry, Some(entry));
    assert_eq!(event.namespace_key, Some(key));
    assert_eq!(event.namespace_object, Some(new_object));
    assert_eq!(event.operation, Some(Operation::Act));
}
