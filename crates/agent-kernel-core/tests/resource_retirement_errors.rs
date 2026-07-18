use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, Operation, OperationSet, ResourceKind,
    ResourceStatus,
};

#[test]
fn retire_resource_requires_rollback_authority_without_mutation() {
    let mut core = KernelCore::<1, 1, 1, 4, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("act capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.retire_resource(agent, capability, resource),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.resources()[0].status, ResourceStatus::Active);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn retire_resource_event_log_full_leaves_resource_active() {
    let mut core = KernelCore::<1, 1, 1, 2, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(2);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");

    assert!(!core.has_event_capacity(1));
    assert_eq!(
        core.can_retire_resource(agent, capability, resource),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(
        core.retire_resource(agent, capability, resource),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.resources()[0].status, ResourceStatus::Active);
    assert_eq!(core.events().len(), 2);
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::CapabilityGranted
    );
}

#[test]
fn retired_resource_rejects_future_grants_and_old_capability_use() {
    let mut core = KernelCore::<1, 1, 2, 6, 0, 1, 0, 0, 0, 0>::new();
    let agent = AgentId::new(3);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let rollback = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");
    let observe = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("observe capability should fit");
    core.retire_resource(agent, rollback, resource)
        .expect("resource should retire");
    let events_before = core.events().len();

    assert_eq!(
        core.grant_capability(agent, resource, OperationSet::only(Operation::Observe)),
        Err(KernelError::ResourceRetired)
    );
    assert_eq!(
        core.observe(agent, observe, resource),
        Err(KernelError::ResourceRetired)
    );
    assert!(core.observations().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn retired_parent_resource_rejects_child_registration() {
    let mut core = KernelCore::<1, 2, 1, 4, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(4);
    core.register_agent(agent).expect("agent should register");
    let parent = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("parent should fit");
    let capability = core
        .grant_capability(agent, parent, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");
    core.retire_resource(agent, capability, parent)
        .expect("parent should retire");

    assert_eq!(
        core.register_resource(ResourceKind::Memory, Some(parent)),
        Err(KernelError::ResourceRetired)
    );
    assert_eq!(core.resources().len(), 1);
}
