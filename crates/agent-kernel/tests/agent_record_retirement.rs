use agent_kernel::AgentKernel;
use agent_kernel_core::{AgentId, EventKind, KernelError, Operation, OperationSet, ResourceKind};

type TestKernel = AgentKernel<4, 1, 1, 16, 0, 0, 0, 0, 0, 0>;

#[test]
fn facade_retires_agent_record_and_reuses_paired_capacity() {
    let mut kernel = TestKernel::new();
    let manager = AgentId::new(1);
    let target = AgentId::new(9);
    let fresh = AgentId::new(15);
    kernel.sys_register_agent(manager).unwrap();
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = kernel
        .sys_grant(manager, resource, OperationSet::only(Operation::Delegate))
        .unwrap();
    kernel
        .sys_register_managed_agent(manager, authority, resource, target)
        .unwrap();
    kernel
        .sys_retire_managed_agent(manager, authority, target)
        .unwrap();

    let retirement = kernel
        .sys_retire_agent_record(manager, authority, target)
        .expect("facade routes paired record retirement");
    kernel
        .sys_register_managed_agent(manager, authority, resource, fresh)
        .expect("fresh identity reuses facade capacity");

    assert_eq!(retirement.agent(), target);
    assert_eq!(retirement.context().agent, target);
    assert_eq!(retirement.management_resource(), resource);
    assert_eq!(retirement.retired_floor(), target);
    assert_eq!(kernel.retired_agent_floor(), target);
    assert_eq!(kernel.agents().last().unwrap().id, fresh);
    assert_eq!(kernel.execution_contexts().last().unwrap().agent, fresh);
    assert_eq!(
        kernel.execution_context(target),
        Err(KernelError::AgentNotFound)
    );
    assert_eq!(
        kernel.events().last().map(|event| event.kind),
        Some(EventKind::AgentRegistered)
    );
}
