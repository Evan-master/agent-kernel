use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, IntentKind, KernelCore, KernelError,
    Operation, OperationSet, ResourceKind, SignalKey, TaskStatus, VerificationRequirement,
};

#[test]
fn emit_signal_run_queue_full_leaves_waiter_waiting() {
    let mut core = KernelCore::<2, 1, 4, 30, 0, 0, 0, 2, 2, 1, 0, 0, 0, 0, 0, 0, 1>::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("owner capability should fit");
    let assignee_runtime_capability = core
        .grant_capability(assignee, resource, OperationSet::only(Operation::Act))
        .expect("assignee runtime capability should fit");
    let image = core
        .register_agent_image(
            assignee,
            assignee_runtime_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    core.launch_agent(
        assignee,
        assignee_runtime_capability,
        resource,
        image,
        AgentEntryKind::Worker,
        None,
    )
    .expect("assignee should launch for resource");

    let first_intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("first intent should declare");
    let first_task = core
        .create_task(owner, owner_capability, first_intent)
        .expect("first task should create");
    core.delegate_task(owner, owner_capability, first_task, assignee)
        .expect("first task should delegate");
    let first_capability = core.tasks()[0]
        .delegated_capability
        .expect("first delegation should derive capability");
    core.accept_task(assignee, first_task)
        .expect("first task should accept");
    core.enqueue_task(assignee, first_task)
        .expect("first task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("first task should dispatch");
    core.wait_task(
        assignee,
        first_capability,
        first_task,
        resource,
        SignalKey::new(4),
    )
    .expect("task should wait");

    let second_intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("second intent should declare");
    let second_task = core
        .create_task(owner, owner_capability, second_intent)
        .expect("second task should create");
    core.delegate_task(owner, owner_capability, second_task, assignee)
        .expect("second task should delegate");
    core.accept_task(assignee, second_task)
        .expect("second task should accept");
    core.enqueue_task(assignee, second_task)
        .expect("second task should fill run queue");
    let events_before = core.events().len();

    assert_eq!(
        core.emit_signal(owner, owner_capability, resource, SignalKey::new(4)),
        Err(KernelError::RunQueueFull)
    );
    assert!(core.waiters()[0].active);
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.events().len(), events_before);
}
