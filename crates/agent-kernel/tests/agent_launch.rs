use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, EventKind, Operation, OperationSet, ResourceKind,
};

#[test]
fn facade_launches_agent_and_exposes_entry() {
    let mut kernel = AgentKernel::<2, 4, 8, 16, 1, 1, 1, 2, 2, 2>::new();
    let agent = AgentId::new(1);

    kernel
        .sys_register_agent(agent)
        .expect("agent should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");

    let event = kernel
        .sys_launch_agent(
            agent,
            capability,
            resource,
            AgentEntryKind::Supervisor,
            None,
        )
        .expect("agent should launch");
    let entry = kernel.agent_entry(agent).expect("agent entry should exist");

    assert_eq!(event.kind, EventKind::AgentLaunched);
    assert_eq!(kernel.agent_entries().len(), 1);
    assert_eq!(entry.agent, agent);
    assert_eq!(entry.resource, resource);
    assert_eq!(entry.capability, capability);
    assert_eq!(entry.kind, AgentEntryKind::Supervisor);
    assert_eq!(entry.intent, None);
}
