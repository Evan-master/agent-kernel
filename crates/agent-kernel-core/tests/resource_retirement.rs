use agent_kernel_core::{
    AgentId, EventKind, KernelCore, Operation, OperationSet, ResourceKind, ResourceStatus,
};

#[test]
fn retire_resource_marks_resource_retired_and_records_event() {
    let mut core = KernelCore::<1, 1, 1, 4, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");

    let event = core
        .retire_resource(agent, capability, resource)
        .expect("rollback authority should retire resource");

    assert_eq!(core.resources()[0].status, ResourceStatus::Retired);
    assert_eq!(event.kind, EventKind::ResourceRetired);
    assert_eq!(event.agent, agent);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(core.events().last(), Some(&event));
}
