use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, IntentKind, Operation, OperationSet,
    ResourceKind, TaskStatus, VerificationRequirement,
};

type TestKernel = AgentKernel<2, 1, 3, 32, 0, 0, 0, 1, 1, 1>;

#[test]
fn completion_readiness_crosses_the_facade_without_mutation() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    kernel.sys_register_agent(owner).unwrap();
    kernel.sys_register_agent(assignee).unwrap();
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::only(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .unwrap();
    let intent = kernel
        .sys_declare_intent(
            owner,
            authority,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = kernel.sys_create_task(owner, authority, intent).unwrap();
    let delegated = kernel
        .sys_delegate_task(owner, authority, task, assignee)
        .unwrap();
    let capability = delegated.capability.unwrap();
    let image = kernel
        .sys_register_agent_image(
            owner,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([8; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(owner, authority, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(assignee, capability, task, image, AgentEntryKind::Worker)
        .unwrap();
    kernel.sys_accept_task(assignee, task).unwrap();
    kernel.sys_enqueue_task(assignee, task).unwrap();
    kernel.sys_dispatch_next(assignee).unwrap();
    let event_count = kernel.events().len();

    assert_eq!(kernel.can_complete_task(assignee, capability, task), Ok(()));
    assert_eq!(kernel.events().len(), event_count);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Running);

    kernel
        .sys_complete_task(assignee, capability, task)
        .unwrap();
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Completed);
}
