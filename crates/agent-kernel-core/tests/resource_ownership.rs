use agent_kernel_core::{AgentId, EventKind, KernelCore, Operation, OperationSet, ResourceKind};

#[test]
fn create_resource_sets_owner_records_events_and_returns_usable_capability() {
    let mut core = KernelCore::<1, 1, 1, 4, 0, 1, 0, 0, 0, 0>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should register");

    let created = core
        .create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::only(Operation::Observe),
        )
        .expect("resource should be created");

    assert_eq!(core.resources()[0].id, created.resource);
    assert_eq!(core.resources()[0].owner, Some(agent));
    assert_eq!(core.events()[1].kind, EventKind::ResourceCreated);
    assert_eq!(core.events()[1].agent, agent);
    assert_eq!(core.events()[1].resource, Some(created.resource));
    assert_eq!(core.events()[1].capability, Some(created.capability));
    assert_eq!(core.events()[2].kind, EventKind::CapabilityGranted);
    assert_eq!(core.events()[2].capability, Some(created.capability));

    core.observe(agent, created.capability, created.resource)
        .expect("initial capability should authorize observe");
    assert_eq!(core.events()[3].kind, EventKind::Observation);
}

#[test]
fn create_child_resource_records_parent_and_owner_when_parent_act_authorizes() {
    let mut core = KernelCore::<1, 2, 2, 5, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(2);
    core.register_agent(agent).expect("agent should register");
    let parent = core
        .create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::only(Operation::Act),
        )
        .expect("parent should be created");

    let child = core
        .create_resource(
            agent,
            ResourceKind::Memory,
            Some((parent.resource, parent.capability)),
            OperationSet::only(Operation::Observe),
        )
        .expect("child should be created");

    assert_eq!(core.resources()[1].id, child.resource);
    assert_eq!(core.resources()[1].parent, Some(parent.resource));
    assert_eq!(core.resources()[1].owner, Some(agent));
    assert_eq!(core.events()[3].kind, EventKind::ResourceCreated);
    assert_eq!(core.events()[4].kind, EventKind::CapabilityGranted);
}
