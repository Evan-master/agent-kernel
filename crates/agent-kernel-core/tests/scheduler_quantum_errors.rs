use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, IntentKind, KernelCore, KernelError,
    Operation, OperationSet, ResourceKind, RunQueueEntry, TaskId, TaskStatus,
    VerificationRequirement,
};

#[derive(Copy, Clone)]
struct AcceptedTask {
    task: TaskId,
}

fn accepted_task<
    const AGENTS: usize,
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
>(
    core: &mut KernelCore<AGENTS, RESOURCES, CAPS, EVENTS, 0, 0, 0, INTENTS, TASKS, RUN_QUEUE>,
    owner: AgentId,
    assignee: AgentId,
) -> AcceptedTask {
    if core.agents().iter().all(|agent| agent.id != owner) {
        core.register_agent(owner).expect("owner should register");
    }
    if core.agents().iter().all(|agent| agent.id != assignee) {
        core.register_agent(assignee)
            .expect("assignee should register");
    }
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("owner capability should fit");
    let intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    let assignee_capability = core
        .tasks()
        .iter()
        .find(|task_record| task_record.id == task)
        .and_then(|task_record| task_record.delegated_capability)
        .expect("delegation should derive assignee capability");
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    core.verify_agent_image(owner, owner_capability, image)
        .expect("image should verify");
    core.launch_task_agent(
        assignee,
        assignee_capability,
        task,
        image,
        AgentEntryKind::Worker,
    )
    .expect("assignee should launch for delegated task");
    core.accept_task(assignee, task)
        .expect("task should be accepted");

    AcceptedTask { task }
}

#[test]
fn dispatch_with_zero_quantum_fails_without_mutation() {
    let mut core = KernelCore::<2, 1, 2, 16, 0, 0, 0, 1, 1, 1>::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let accepted = accepted_task(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");
    let events_before = core.events().len();
    let queue_before = core.run_queue()[0];

    assert_eq!(
        core.dispatch_next_with_quantum(assignee, 0),
        Err(KernelError::TaskQuantumInvalid)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.tasks()[0].quantum_remaining, 0);
    assert_eq!(core.run_queue(), &[queue_before]);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn tick_rejects_non_running_task_without_mutation() {
    let mut core = KernelCore::<2, 1, 2, 16, 0, 0, 0, 1, 1, 1>::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let accepted = accepted_task(&mut core, owner, assignee);
    let events_before = core.events().len();

    assert_eq!(
        core.tick_task(assignee, accepted.task),
        Err(KernelError::TaskNotRunnable)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.tasks()[0].run_ticks, 0);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn tick_rejects_wrong_agent_without_mutation() {
    let mut core = KernelCore::<3, 1, 2, 20, 0, 0, 0, 1, 1, 1>::new();
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
    let wrong_agent = AgentId::new(7);
    let accepted = accepted_task(&mut core, owner, assignee);
    core.register_agent(wrong_agent)
        .expect("wrong agent should register");
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");
    let events_before = core.events().len();

    assert_eq!(
        core.tick_task(wrong_agent, accepted.task),
        Err(KernelError::TaskNotRunnable)
    );
    assert_eq!(core.tasks()[0].run_ticks, 0);
    assert_eq!(core.tasks()[0].quantum_remaining, 2);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn tick_event_log_full_leaves_running_task_unchanged() {
    let mut core = KernelCore::<2, 1, 2, 14, 0, 0, 0, 1, 1, 1>::new();
    let owner = AgentId::new(8);
    let assignee = AgentId::new(9);
    let accepted = accepted_task(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("dispatch should consume final event slot");

    assert_eq!(
        core.tick_task(assignee, accepted.task),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].run_ticks, 0);
    assert_eq!(core.tasks()[0].quantum_remaining, 2);
    assert_eq!(core.events().len(), 14);
}

#[test]
fn quantum_expiry_run_queue_full_leaves_running_task_unchanged() {
    let mut core = KernelCore::<3, 2, 4, 32, 0, 0, 0, 2, 2, 1>::new();
    let owner = AgentId::new(10);
    let first_agent = AgentId::new(11);
    let second_agent = AgentId::new(12);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);
    core.enqueue_task(first_agent, first.task)
        .expect("first task should enqueue");
    core.dispatch_next_with_quantum(first_agent, 1)
        .expect("first task should dispatch");
    core.enqueue_task(second_agent, second.task)
        .expect("second task should fill queue");
    let events_before = core.events().len();

    assert_eq!(
        core.tick_task(first_agent, first.task),
        Err(KernelError::RunQueueFull)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].run_ticks, 0);
    assert_eq!(core.tasks()[0].quantum_remaining, 1);
    assert_eq!(
        core.run_queue(),
        &[RunQueueEntry {
            task: second.task,
            agent: second_agent,
        }]
    );
    assert_eq!(core.events().len(), events_before);
}
