use agent_kernel::AgentKernel;
use agent_kernel_core::{AgentId, AgentStatus, EventKind, Operation, OperationSet, ResourceKind};

type TestKernel = AgentKernel<2, 1, 1, 8, 0, 0, 0, 0, 0, 0>;

#[test]
fn managed_agent_syscalls_expose_authorized_lifecycle() {
    let mut kernel = TestKernel::new();
    let manager = AgentId::new(1);
    let target = AgentId::new(9);
    kernel
        .sys_register_agent(manager)
        .expect("manager should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(manager, resource, OperationSet::only(Operation::Delegate))
        .expect("management capability should fit");

    let registered = kernel
        .sys_register_managed_agent(manager, capability, resource, target)
        .expect("managed agent should register");
    let suspended = kernel
        .sys_suspend_managed_agent(manager, capability, target)
        .expect("managed agent should suspend");
    let resumed = kernel
        .sys_resume_managed_agent(manager, capability, target)
        .expect("managed agent should resume");
    let retired = kernel
        .sys_retire_managed_agent(manager, capability, target)
        .expect("managed agent should retire");

    assert_eq!(registered.kind, EventKind::AgentRegistered);
    assert_eq!(suspended.kind, EventKind::AgentSuspended);
    assert_eq!(resumed.kind, EventKind::AgentResumed);
    assert_eq!(retired.kind, EventKind::AgentRetired);
    assert_eq!(retired.agent, manager);
    assert_eq!(retired.target_agent, Some(target));
    assert_eq!(retired.resource, Some(resource));
    assert_eq!(retired.capability, Some(capability));
    assert_eq!(kernel.agents()[1].status, AgentStatus::Retired);
    assert_eq!(kernel.agents()[1].manager, Some(manager));
    assert_eq!(kernel.agents()[1].management_resource, Some(resource));
}
