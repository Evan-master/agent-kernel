use agent_kernel::AgentKernel;
use agent_kernel_core::{AgentId, EventKind, Operation, OperationSet, ResourceKind};

type TestKernel = AgentKernel<2, 2, 2, 4, 1, 1, 1, 0, 1, 1>;

#[test]
fn sys_grant_records_capability_granted_event() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let operations = OperationSet::empty()
        .with(Operation::Observe)
        .with(Operation::Act);

    let capability = kernel
        .sys_grant(agent, resource, operations)
        .expect("grant should fit");

    let events = kernel.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::CapabilityGranted);
    assert_eq!(events[0].agent, agent);
    assert_eq!(events[0].resource, Some(resource));
    assert_eq!(events[0].capability, Some(capability));
    assert_eq!(events[0].operations, operations);
}
