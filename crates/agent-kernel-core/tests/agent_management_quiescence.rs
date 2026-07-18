use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, AgentStatus, KernelCore,
    KernelError, Operation, OperationSet, ResourceKind,
};

type TestCore = KernelCore<2, 1, 2, 16, 0, 0, 0, 0, 0, 0>;

#[test]
fn launched_managed_agent_rejects_lifecycle_without_mutation() {
    let mut core = TestCore::new();
    let manager = AgentId::new(1);
    let target = AgentId::new(9);
    core.register_agent(manager)
        .expect("manager should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let authority = core
        .grant_capability(
            manager,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify)
                .with(Operation::Delegate),
        )
        .expect("manager authority should fit");
    core.register_managed_agent(manager, authority, resource, target)
        .expect("managed target should register");
    let target_capability = core
        .grant_capability(target, resource, OperationSet::only(Operation::Act))
        .expect("target authority should fit");
    let image = core
        .register_agent_image(
            manager,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([7; 32]),
            1,
            1,
        )
        .expect("image should register");
    core.verify_agent_image(manager, authority, image)
        .expect("image should verify");
    core.launch_agent(
        target,
        target_capability,
        resource,
        image,
        AgentEntryKind::Worker,
        None,
    )
    .expect("target should launch");
    let events_before = core.events().len();

    assert_eq!(
        core.suspend_managed_agent(manager, authority, target),
        Err(KernelError::AgentManagementBusy)
    );
    assert_eq!(core.agents()[1].status, AgentStatus::Active);
    assert_eq!(core.events().len(), events_before);
}
