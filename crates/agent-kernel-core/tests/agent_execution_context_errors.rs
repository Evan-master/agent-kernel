use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, IntentKind,
    KernelCore, KernelError, Operation, OperationSet, ResourceKind, TaskStatus,
    VerificationRequirement,
};

type TestCore = KernelCore<3, 3, 8, 48, 0, 0, 0, 8, 8, 8>;

fn prepare_two_accepted_tasks(
    core: &mut TestCore,
) -> (
    AgentId,
    agent_kernel_core::TaskId,
    agent_kernel_core::TaskId,
) {
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("capability should fit");
    let assignee_capability = core
        .grant_capability(assignee, resource, OperationSet::only(Operation::Act))
        .expect("assignee root capability should fit");
    let image = core
        .register_agent_image(
            assignee,
            assignee_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    core.launch_agent(
        assignee,
        assignee_capability,
        resource,
        image,
        AgentEntryKind::Worker,
        None,
    )
    .expect("assignee should launch for workspace tasks");

    let first_intent = core
        .declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("first intent should fit");
    let first = core
        .create_task(owner, capability, first_intent)
        .expect("first task should fit");
    core.delegate_task(owner, capability, first, assignee)
        .expect("first task should delegate");
    core.accept_task(assignee, first)
        .expect("first task should be accepted");

    let second_intent = core
        .declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("second intent should fit");
    let second = core
        .create_task(owner, capability, second_intent)
        .expect("second task should fit");
    core.delegate_task(owner, capability, second, assignee)
        .expect("second task should delegate");
    core.accept_task(assignee, second)
        .expect("second task should be accepted");

    (assignee, first, second)
}

#[test]
fn dispatch_rejects_busy_execution_context_without_mutation() {
    let mut core = TestCore::new();
    let (assignee, first, second) = prepare_two_accepted_tasks(&mut core);

    core.enqueue_task(assignee, first)
        .expect("first task should enqueue");
    core.enqueue_task(assignee, second)
        .expect("second task should enqueue");
    core.dispatch_next_with_quantum(assignee, 4)
        .expect("first task should dispatch");
    let events_before = core.events().len();

    assert_eq!(
        core.dispatch_next_with_quantum(assignee, 1),
        Err(KernelError::ExecutionContextBusy)
    );

    let context = core.execution_context(assignee).unwrap();
    assert_eq!(context.state, AgentExecutionState::Running);
    assert_eq!(context.task, Some(first));
    assert_eq!(context.quantum_remaining, 4);
    assert_eq!(core.run_queue().len(), 1);
    assert_eq!(core.run_queue()[0].task, second);
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[1].status, TaskStatus::Accepted);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn register_agent_event_log_full_leaves_no_execution_context() {
    let mut core = KernelCore::<1, 0, 0, 0, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(3);

    assert_eq!(core.register_agent(agent), Err(KernelError::EventLogFull));
    assert!(core.agents().is_empty());
    assert!(core.execution_contexts().is_empty());
}

#[test]
fn register_agent_store_full_leaves_no_execution_context() {
    let mut core = KernelCore::<0, 0, 0, 1, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(4);

    assert_eq!(core.register_agent(agent), Err(KernelError::AgentStoreFull));
    assert!(core.agents().is_empty());
    assert!(core.execution_contexts().is_empty());
}
