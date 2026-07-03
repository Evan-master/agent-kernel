use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, IntentKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind, RunQueueEntry, TaskId, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<4, 4, 6, 32, 4, 2, 2, 6, 6, 4>;

#[derive(Copy, Clone)]
struct AcceptedTask {
    task: TaskId,
    owner_capability: CapabilityId,
    assignee_capability: CapabilityId,
}

fn accepted_task<
    const AGENTS: usize,
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const ACTIONS: usize,
    const OBSERVATIONS: usize,
    const CHECKPOINTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
>(
    core: &mut KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
    >,
    owner: AgentId,
    assignee: AgentId,
) -> TaskId {
    accepted_task_with_capabilities(core, owner, assignee).task
}

fn accepted_task_with_capabilities<
    const AGENTS: usize,
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const ACTIONS: usize,
    const OBSERVATIONS: usize,
    const CHECKPOINTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
>(
    core: &mut KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
    >,
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
                .with(Operation::Verify)
                .with(Operation::Rollback),
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
    let delegation = core
        .delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    let assignee_capability = delegation
        .capability
        .expect("delegation should derive assignee capability");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    AcceptedTask {
        task,
        owner_capability,
        assignee_capability,
    }
}

#[test]
fn enqueue_accepted_task_records_fifo_entry() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let task = accepted_task(&mut core, owner, assignee);
    let events_before_enqueue = core.events().len();

    let event = core
        .enqueue_task(assignee, task)
        .expect("accepted task should enqueue");

    assert_eq!(event.kind, EventKind::TaskQueued);
    assert_eq!(event.task, Some(task));
    assert_eq!(event.agent, assignee);
    assert_eq!(
        core.run_queue(),
        &[RunQueueEntry {
            task,
            agent: assignee
        }]
    );
    assert_eq!(core.events().len(), events_before_enqueue + 1);
}

#[test]
fn dispatch_next_pops_oldest_task_and_records_event() {
    let mut core = TestCore::new();
    let owner = AgentId::new(3);
    let first_agent = AgentId::new(4);
    let second_agent = AgentId::new(5);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);
    core.enqueue_task(first_agent, first)
        .expect("first task should enqueue");
    core.enqueue_task(second_agent, second)
        .expect("second task should enqueue");

    let dispatched = core
        .dispatch_next(first_agent)
        .expect("first agent should dispatch first queued task");

    assert_eq!(dispatched, first);
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(
        core.run_queue(),
        &[RunQueueEntry {
            task: second,
            agent: second_agent,
        }]
    );
    let last = core.events().last().expect("dispatch should record event");
    assert_eq!(last.kind, EventKind::TaskDispatched);
    assert_eq!(last.task, Some(first));
    assert_eq!(last.agent, first_agent);
}

#[test]
fn scheduler_rejects_invalid_queue_operations_without_state_changes() {
    let mut core = TestCore::new();
    let owner = AgentId::new(6);
    let assignee = AgentId::new(7);
    let wrong_agent = AgentId::new(8);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let intent = core
        .declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let created = core
        .create_task(owner, capability, intent)
        .expect("task should be created");
    let accepted = accepted_task(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted)
        .expect("accepted task should enqueue");
    let queue_before = *core.run_queue().first().expect("queue should have entry");
    let events_before = core.events().len();

    assert_eq!(
        core.enqueue_task(owner, created),
        Err(KernelError::TaskNotRunnable)
    );
    assert_eq!(
        core.enqueue_task(wrong_agent, accepted),
        Err(KernelError::TaskNotRunnable)
    );
    assert_eq!(
        core.enqueue_task(assignee, accepted),
        Err(KernelError::TaskAlreadyQueued)
    );
    assert_eq!(
        core.dispatch_next(wrong_agent),
        Err(KernelError::TaskNotRunnable)
    );
    assert_eq!(core.run_queue(), &[queue_before]);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn enqueue_returns_run_queue_full_when_capacity_is_exhausted() {
    let mut core = KernelCore::<3, 4, 6, 32, 4, 2, 2, 4, 4, 1>::new();
    let owner = AgentId::new(9);
    let first_agent = AgentId::new(10);
    let second_agent = AgentId::new(11);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);

    core.enqueue_task(first_agent, first)
        .expect("first task should enqueue");
    let result = core.enqueue_task(second_agent, second);

    assert_eq!(result, Err(KernelError::RunQueueFull));
    assert_eq!(
        core.run_queue(),
        &[RunQueueEntry {
            task: first,
            agent: first_agent,
        }]
    );
}

#[test]
fn dispatch_from_empty_queue_returns_run_queue_empty() {
    let mut core = TestCore::new();

    let result = core.dispatch_next(AgentId::new(12));

    assert_eq!(result, Err(KernelError::RunQueueEmpty));
    assert!(core.run_queue().is_empty());
    assert!(core.events().is_empty());
}

#[test]
fn yield_task_requeues_running_task_as_accepted_at_back() {
    let mut core = TestCore::new();
    let owner = AgentId::new(13);
    let first_agent = AgentId::new(14);
    let second_agent = AgentId::new(15);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);
    core.enqueue_task(first_agent, first)
        .expect("first task should enqueue");
    core.enqueue_task(second_agent, second)
        .expect("second task should enqueue");
    core.dispatch_next(first_agent)
        .expect("first task should dispatch");

    let event = core
        .yield_task(first_agent, first)
        .expect("running task should yield into queue");

    assert_eq!(event.kind, EventKind::TaskYielded);
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(
        core.run_queue(),
        &[
            RunQueueEntry {
                task: second,
                agent: second_agent,
            },
            RunQueueEntry {
                task: first,
                agent: first_agent,
            },
        ]
    );
}

#[test]
fn completing_accepted_task_before_dispatch_is_rejected_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(16);
    let assignee = AgentId::new(17);
    let accepted = accepted_task_with_capabilities(&mut core, owner, assignee);
    let events_before = core.events().len();

    let result = core.complete_task(assignee, accepted.assignee_capability, accepted.task);

    assert_eq!(result, Err(KernelError::TaskStatusMismatch));
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn completing_running_task_records_completed_status() {
    let mut core = TestCore::new();
    let owner = AgentId::new(18);
    let assignee = AgentId::new(19);
    let accepted = accepted_task_with_capabilities(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");
    core.dispatch_next(assignee)
        .expect("accepted task should dispatch");

    let event = core
        .complete_task(assignee, accepted.assignee_capability, accepted.task)
        .expect("running task should complete");

    assert_eq!(event.kind, EventKind::TaskCompleted);
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
}

#[test]
fn yielding_accepted_task_without_dispatch_is_rejected_without_state_changes() {
    let mut core = TestCore::new();
    let owner = AgentId::new(20);
    let assignee = AgentId::new(21);
    let accepted = accepted_task_with_capabilities(&mut core, owner, assignee);
    let events_before = core.events().len();

    let result = core.yield_task(assignee, accepted.task);

    assert_eq!(result, Err(KernelError::TaskNotRunnable));
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert!(core.run_queue().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn cancelling_running_task_marks_cancelled_and_blocks_completion() {
    let mut core = TestCore::new();
    let owner = AgentId::new(22);
    let assignee = AgentId::new(23);
    let accepted = accepted_task_with_capabilities(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");
    core.dispatch_next(assignee)
        .expect("accepted task should dispatch");

    let event = core
        .cancel_task(owner, accepted.owner_capability, accepted.task)
        .expect("running task should cancel");

    assert_eq!(event.kind, EventKind::TaskCancelled);
    assert_eq!(core.tasks()[0].status, TaskStatus::Cancelled);
    assert_eq!(
        core.complete_task(assignee, accepted.assignee_capability, accepted.task),
        Err(KernelError::TaskStatusMismatch)
    );
}
