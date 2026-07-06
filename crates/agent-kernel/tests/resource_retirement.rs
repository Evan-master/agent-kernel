use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, KernelError, Operation, OperationSet, ResourceKind, ResourceStatus,
};

type TestKernel = AgentKernel<1, 1, 1, 4, 0, 0, 0, 0, 0, 0>;

#[test]
fn resource_retirement_syscall_retires_resource_and_blocks_future_grants() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
    kernel
        .sys_register_agent(agent)
        .expect("agent should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Service, None)
        .expect("service resource should fit");
    let capability = kernel
        .sys_grant(agent, resource, OperationSet::only(Operation::Rollback))
        .expect("rollback capability should fit");

    let event = kernel
        .sys_retire_resource(agent, capability, resource)
        .expect("resource should retire");

    assert_eq!(event.kind, EventKind::ResourceRetired);
    assert_eq!(kernel.resources()[0].status, ResourceStatus::Retired);
    assert_eq!(
        kernel.sys_grant(agent, resource, OperationSet::only(Operation::Observe)),
        Err(KernelError::ResourceRetired)
    );
}
