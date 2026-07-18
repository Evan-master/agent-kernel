use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, IntentKind, KernelCore, Operation,
    OperationSet, ResourceKind, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<2, 1, 3, 32, 0, 0, 0, 1, 1, 1>;

#[test]
fn completion_readiness_is_read_only_and_matches_commit() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(assignee).unwrap();
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = core
        .grant_capability(
            owner,
            resource,
            OperationSet::only(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .unwrap();
    let intent = core
        .declare_intent(
            owner,
            authority,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = core.create_task(owner, authority, intent).unwrap();
    let delegated = core
        .delegate_task(owner, authority, task, assignee)
        .unwrap();
    let capability = delegated.capability.unwrap();
    let image = core
        .register_agent_image(
            owner,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([7; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(owner, authority, image).unwrap();
    core.launch_task_agent(assignee, capability, task, image, AgentEntryKind::Worker)
        .unwrap();
    core.accept_task(assignee, task).unwrap();
    core.enqueue_task(assignee, task).unwrap();
    core.dispatch_next(assignee).unwrap();
    let event_count = core.events().len();

    assert_eq!(core.can_complete_task(assignee, capability, task), Ok(()));
    assert_eq!(core.events().len(), event_count);
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);

    core.complete_task(assignee, capability, task).unwrap();
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
}
