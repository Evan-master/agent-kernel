use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, IntentKind, Operation,
    OperationSet, ResourceKind, SignalKey, TaskStatus, VerificationRequirement, WaiterId,
};

type TestKernel = AgentKernel<2, 1, 2, 20, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn signal_syscalls_wait_wake_redispatch_and_complete_task() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    kernel
        .sys_register_agent(owner)
        .expect("owner should register");
    kernel
        .sys_register_agent(assignee)
        .expect("assignee should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("owner capability should fit");
    let intent = kernel
        .sys_declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    let task = kernel
        .sys_create_task(owner, owner_capability, intent)
        .expect("task should create");
    kernel
        .sys_delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    let image = kernel
        .sys_register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    kernel
        .sys_verify_agent_image(owner, owner_capability, image)
        .expect("image should verify");
    kernel
        .sys_launch_task_agent(
            assignee,
            assignee_capability,
            task,
            image,
            AgentEntryKind::Worker,
        )
        .expect("assignee should launch for delegated task");
    kernel
        .sys_accept_task(assignee, task)
        .expect("task should accept");
    kernel
        .sys_enqueue_task(assignee, task)
        .expect("task should enqueue");
    kernel
        .sys_dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");
    let signal = SignalKey::new(7);

    let waiter = kernel
        .sys_wait_task(assignee, assignee_capability, task, resource, signal)
        .expect("task should wait");
    let outcome = kernel
        .sys_emit_signal(owner, owner_capability, resource, signal)
        .expect("signal should wake task");
    let redispatched = kernel
        .sys_dispatch_next_with_quantum(assignee, 1)
        .expect("woken task should dispatch");
    kernel
        .sys_complete_task(assignee, assignee_capability, task)
        .expect("woken task should complete");

    assert_eq!(waiter, WaiterId::new(1));
    assert_eq!(outcome.signal_event.kind, EventKind::SignalEmitted);
    assert_eq!(outcome.woken_task, Some(task));
    assert_eq!(redispatched, task);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Completed);
    assert_eq!(kernel.waiters()[0].id, waiter);
    assert!(!kernel.waiters()[0].active);
    assert_eq!(kernel.events()[14].kind, EventKind::TaskWaiting);
    assert_eq!(kernel.events()[15].kind, EventKind::SignalEmitted);
    assert_eq!(kernel.events()[16].kind, EventKind::TaskWoken);
}
