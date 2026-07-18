use agent_kernel_core::{AgentId, KernelCore, KernelError, Operation, OperationSet, ResourceKind};

type TestCore = KernelCore<2, 1, 1, 12, 0, 0, 0, 0, 0, 0>;

#[test]
fn managed_lifecycle_rejects_invalid_and_terminal_transitions_atomically() {
    let mut core = TestCore::new();
    let manager = AgentId::new(1);
    let target = AgentId::new(9);
    core.register_agent(manager)
        .expect("manager should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(manager, resource, OperationSet::only(Operation::Delegate))
        .expect("authority should fit");
    core.register_managed_agent(manager, capability, resource, target)
        .expect("managed target should register");

    assert_eq!(
        core.resume_managed_agent(manager, capability, target),
        Err(KernelError::AgentStatusMismatch)
    );
    core.suspend_managed_agent(manager, capability, target)
        .expect("active target should suspend");
    let suspended_events = core.events().len();
    assert_eq!(
        core.suspend_managed_agent(manager, capability, target),
        Err(KernelError::AgentStatusMismatch)
    );
    assert_eq!(core.events().len(), suspended_events);

    core.resume_managed_agent(manager, capability, target)
        .expect("suspended target should resume");
    core.retire_managed_agent(manager, capability, target)
        .expect("active target should retire");
    let retired_events = core.events().len();

    assert_eq!(
        core.suspend_managed_agent(manager, capability, target),
        Err(KernelError::AgentRetired)
    );
    assert_eq!(
        core.resume_managed_agent(manager, capability, target),
        Err(KernelError::AgentRetired)
    );
    assert_eq!(
        core.retire_managed_agent(manager, capability, target),
        Err(KernelError::AgentRetired)
    );
    assert_eq!(core.events().len(), retired_events);
}
