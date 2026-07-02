use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, Operation, OperationSet, ResourceKind,
    RunQueueEntry, TaskId,
};

type TestCore = KernelCore<4, 6, 32, 6, 4>;

fn accepted_task<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
>(
    core: &mut KernelCore<RESOURCES, CAPS, EVENTS, TASKS, RUN_QUEUE>,
    owner: AgentId,
    assignee: AgentId,
) -> TaskId {
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
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    task
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
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let created = core
        .create_task(owner, capability, resource)
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
    let mut core = KernelCore::<4, 6, 32, 4, 1>::new();
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
fn yield_task_requeues_accepted_task_at_back() {
    let mut core = TestCore::new();
    let owner = AgentId::new(13);
    let first_agent = AgentId::new(14);
    let second_agent = AgentId::new(15);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);
    core.enqueue_task(second_agent, second)
        .expect("second task should enqueue first");

    let event = core
        .yield_task(first_agent, first)
        .expect("accepted task should yield into queue");

    assert_eq!(event.kind, EventKind::TaskYielded);
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
