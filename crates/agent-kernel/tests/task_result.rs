use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, IntentKind, Operation,
    OperationSet, ResourceKind, TaskResult, VerificationRequirement,
};

type TestKernel = AgentKernel<2, 1, 2, 24, 0, 0, 0, 1, 1, 1>;

#[test]
fn task_result_syscall_exposes_running_task_result_and_event() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    kernel.sys_register_agent(owner).unwrap();
    kernel.sys_register_agent(assignee).unwrap();
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let owner_capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .unwrap();
    let intent = kernel
        .sys_declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = kernel
        .sys_create_task(owner, owner_capability, intent)
        .unwrap();
    let capability = kernel
        .sys_delegate_task(owner, owner_capability, task, assignee)
        .unwrap()
        .capability
        .unwrap();
    let image = kernel
        .sys_register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([0x77; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(owner, owner_capability, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(assignee, capability, task, image, AgentEntryKind::Worker)
        .unwrap();
    kernel.sys_accept_task(assignee, task).unwrap();
    kernel.sys_enqueue_task(assignee, task).unwrap();
    kernel.sys_dispatch_next(assignee).unwrap();
    let result = TaskResult {
        code: 0x1234,
        value: 0x5678,
    };

    let event = kernel
        .sys_submit_task_result(assignee, capability, task, result)
        .unwrap();

    assert_eq!(event.kind, EventKind::TaskResultSubmitted);
    assert_eq!(event.task_result, Some(result));
    assert_eq!(kernel.tasks()[0].result, Some(result));
}
