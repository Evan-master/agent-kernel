use agent_kernel_core::{
    AgentId, FaultKind, FaultPolicyAction, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind,
};

type InstallCore<const FAULT_POLICIES: usize> =
    KernelCore<1, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, FAULT_POLICIES>;

#[test]
fn install_fault_policy_requires_rollback_authority_without_mutation() {
    let mut core = InstallCore::<1>::new();
    let owner = AgentId::new(6);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.install_fault_policy(
            owner,
            capability,
            resource,
            FaultKind::ExecutionTrap,
            FaultPolicyAction::RouteToHandler,
        ),
        Err(KernelError::OperationDenied)
    );
    assert!(core.fault_policies().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn install_fault_policy_rejects_duplicate_without_mutation() {
    let mut core = InstallCore::<2>::new();
    let owner = AgentId::new(7);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Rollback))
        .expect("capability should fit");
    core.install_fault_policy(
        owner,
        capability,
        resource,
        FaultKind::ExecutionTrap,
        FaultPolicyAction::RouteToHandler,
    )
    .expect("first install should succeed");
    let events_before = core.events().len();

    assert_eq!(
        core.install_fault_policy(
            owner,
            capability,
            resource,
            FaultKind::ExecutionTrap,
            FaultPolicyAction::RecoverTask,
        ),
        Err(KernelError::FaultPolicyAlreadyExists)
    );
    assert_eq!(core.fault_policies().len(), 1);
    assert_eq!(core.events().len(), events_before);
}
