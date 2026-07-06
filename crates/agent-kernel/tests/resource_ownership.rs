use agent_kernel::AgentKernel;
use agent_kernel_core::{AgentId, EventKind, Operation, OperationSet, ResourceKind};

type TestKernel = AgentKernel<1, 1, 1, 4, 0, 1, 0, 0, 0, 0>;

#[test]
fn sys_create_resource_returns_owned_resource_and_usable_capability() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
    kernel
        .sys_register_agent(agent)
        .expect("agent should register");

    let created = kernel
        .sys_create_resource(
            agent,
            ResourceKind::Workspace,
            None,
            OperationSet::only(Operation::Observe),
        )
        .expect("resource should be created");
    let event = kernel
        .sys_observe(agent, created.capability, created.resource)
        .expect("created capability should authorize observe");

    assert_eq!(kernel.resources()[0].owner, Some(agent));
    assert_eq!(kernel.events()[1].kind, EventKind::ResourceCreated);
    assert_eq!(kernel.events()[2].kind, EventKind::CapabilityGranted);
    assert_eq!(event.kind, EventKind::Observation);
}
