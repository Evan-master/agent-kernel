use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, IntentKind, MessageKind,
    MessagePayload, MessageReceiveOutcome, Operation, OperationSet, ResourceKind, TaskStatus,
    VerificationRequirement, WaiterId,
};

type TestKernel = AgentKernel<2, 1, 2, 24, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1>;

#[test]
fn facade_waits_wakes_and_finishes_the_original_receive() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let recipient = AgentId::new(2);
    kernel.sys_register_agent(owner).unwrap();
    kernel.sys_register_agent(recipient).unwrap();
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
    kernel
        .sys_delegate_task(owner, owner_capability, task, recipient)
        .unwrap();
    let task_capability = kernel.tasks()[0].delegated_capability.unwrap();
    let image = kernel
        .sys_register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([2; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(owner, owner_capability, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(
            recipient,
            task_capability,
            task,
            image,
            AgentEntryKind::Worker,
        )
        .unwrap();
    kernel.sys_accept_task(recipient, task).unwrap();
    kernel.sys_enqueue_task(recipient, task).unwrap();
    kernel.sys_dispatch_next_with_quantum(recipient, 1).unwrap();

    let wait = kernel
        .sys_receive_or_wait_message(recipient, task_capability, task)
        .unwrap();
    let message = kernel
        .sys_send_message(
            owner,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .unwrap();
    kernel.sys_dispatch_next_with_quantum(recipient, 1).unwrap();
    let received = kernel.sys_receive_message(recipient).unwrap();

    assert_eq!(wait, MessageReceiveOutcome::Waiting(WaiterId::new(1)));
    assert_eq!(received, message);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Running);
    assert!(!kernel.waiters()[0].active);
    assert_eq!(
        kernel.events()[kernel.events().len() - 2].kind,
        EventKind::TaskDispatched
    );
    assert_eq!(
        kernel.events().last().unwrap().kind,
        EventKind::MessageReceived
    );
}
