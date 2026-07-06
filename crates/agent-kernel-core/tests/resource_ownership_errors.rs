use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind,
};

#[test]
fn create_resource_rejects_inactive_agent_without_mutation() {
    let mut core = KernelCore::<0, 1, 1, 2, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(1);

    assert_eq!(
        core.create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::only(Operation::Observe)
        ),
        Err(KernelError::AgentNotFound)
    );
    assert!(core.resources().is_empty());
    assert!(core.events().is_empty());
}

#[test]
fn create_child_resource_requires_parent_act_authority_without_mutation() {
    let mut core = KernelCore::<1, 2, 2, 5, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(2);
    core.register_agent(agent).expect("agent should register");
    let parent = core
        .create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::only(Operation::Observe),
        )
        .expect("parent should be created");
    let events_before = core.events().len();

    assert_eq!(
        core.create_resource(
            agent,
            ResourceKind::Memory,
            Some((parent.resource, parent.capability)),
            OperationSet::only(Operation::Observe),
        ),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.resources().len(), 1);
    assert_eq!(core.events().len(), events_before);
    assert_eq!(
        core.observe(agent, CapabilityId::new(2), parent.resource),
        Err(KernelError::CapabilityNotFound)
    );
}

#[test]
fn create_resource_event_log_full_leaves_no_resource_or_capability() {
    let mut core = KernelCore::<1, 1, 1, 1, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(3);
    core.register_agent(agent).expect("agent should register");

    assert_eq!(
        core.create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::only(Operation::Observe)
        ),
        Err(KernelError::EventLogFull)
    );
    assert!(core.resources().is_empty());
    assert_eq!(core.events().len(), 1);
}

#[test]
fn create_resource_capability_store_full_leaves_no_resource() {
    let mut core = KernelCore::<1, 1, 0, 3, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(4);
    core.register_agent(agent).expect("agent should register");

    assert_eq!(
        core.create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::only(Operation::Observe)
        ),
        Err(KernelError::CapabilityStoreFull)
    );
    assert!(core.resources().is_empty());
    assert_eq!(core.events().len(), 1);
}

#[test]
fn create_resource_store_full_leaves_no_capability_or_event() {
    let mut core = KernelCore::<1, 0, 1, 3, 0, 1, 0, 0, 0, 0>::new();
    let agent = AgentId::new(5);
    core.register_agent(agent).expect("agent should register");

    assert_eq!(
        core.create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::only(Operation::Observe)
        ),
        Err(KernelError::ResourceStoreFull)
    );
    assert_eq!(core.events().len(), 1);
    assert_eq!(
        core.observe(
            agent,
            CapabilityId::new(1),
            agent_kernel_core::ResourceId::new(1)
        ),
        Err(KernelError::ResourceNotFound)
    );
    assert!(core.observations().is_empty());
}

#[test]
fn create_child_resource_rejects_retired_parent_without_mutation() {
    let mut core = KernelCore::<1, 2, 2, 6, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(6);
    core.register_agent(agent).expect("agent should register");
    let parent = core
        .create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("parent should be created");
    core.retire_resource(agent, parent.capability, parent.resource)
        .expect("parent should retire");
    let events_before = core.events().len();

    assert_eq!(
        core.create_resource(
            agent,
            ResourceKind::Memory,
            Some((parent.resource, parent.capability)),
            OperationSet::only(Operation::Observe),
        ),
        Err(KernelError::ResourceRetired)
    );
    assert_eq!(core.resources().len(), 1);
    assert_eq!(core.events().len(), events_before);
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::ResourceRetired
    );
}
