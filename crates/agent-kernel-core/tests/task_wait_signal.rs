use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, EventKind, IntentKind,
    KernelCore, Operation, OperationSet, ResourceId, ResourceKind, SignalKey, TaskId, TaskStatus,
    VerificationRequirement, WaiterId,
};

type SignalCore<const EVENTS: usize, const RUN_QUEUE: usize, const WAITERS: usize> =
    KernelCore<2, 1, 2, EVENTS, 0, 0, 0, 1, 1, RUN_QUEUE, 0, 0, 0, 0, 0, 0, WAITERS>;

#[derive(Copy, Clone)]
struct RunningTask {
    owner: AgentId,
    assignee: AgentId,
    owner_capability: CapabilityId,
    assignee_capability: CapabilityId,
    resource: ResourceId,
    task: TaskId,
}

fn running_task<const EVENTS: usize, const RUN_QUEUE: usize, const WAITERS: usize>(
    core: &mut SignalCore<EVENTS, RUN_QUEUE, WAITERS>,
) -> RunningTask {
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
        .expect("task should delegate");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive task capability");
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
    core.launch_task_agent(
        assignee,
        assignee_capability,
        task,
        image,
        AgentEntryKind::Worker,
    )
    .expect("assignee should launch for delegated task");
    core.accept_task(assignee, task)
        .expect("task should accept");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");
    RunningTask {
        owner,
        assignee,
        owner_capability,
        assignee_capability,
        resource,
        task,
    }
}

#[test]
fn wait_task_records_waiter_and_blocks_running_task() {
    let mut core = SignalCore::<16, 1, 1>::new();
    let running = running_task(&mut core);
    let signal = SignalKey::new(9);

    let waiter = core
        .wait_task(
            running.assignee,
            running.assignee_capability,
            running.task,
            running.resource,
            signal,
        )
        .expect("running task should wait");

    assert_eq!(waiter, WaiterId::new(1));
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.waiters().len(), 1);
    assert_eq!(core.waiters()[0].task, running.task);
    assert_eq!(core.waiters()[0].signal, signal);
    assert!(core.waiters()[0].active);
    let event = core.events().last().expect("wait should record event");
    assert_eq!(event.kind, EventKind::TaskWaiting);
    assert_eq!(event.task, Some(running.task));
    assert_eq!(event.waiter, Some(waiter));
    assert_eq!(event.signal, Some(signal));
}

#[test]
fn emit_signal_without_waiter_records_signal_only() {
    let mut core = KernelCore::<1, 1, 1, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0>::new();
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let signal = SignalKey::new(11);

    let outcome = core
        .emit_signal(owner, capability, resource, signal)
        .expect("authorized signal should emit");

    assert_eq!(outcome.signal_event.kind, EventKind::SignalEmitted);
    assert_eq!(outcome.signal_event.signal, Some(signal));
    assert_eq!(outcome.woken_task, None);
    assert_eq!(outcome.wake_event, None);
}

#[test]
fn emit_signal_wakes_oldest_matching_waiter_and_enqueues_task() {
    let mut core = SignalCore::<20, 1, 1>::new();
    let running = running_task(&mut core);
    let signal = SignalKey::new(12);
    let waiter = core
        .wait_task(
            running.assignee,
            running.assignee_capability,
            running.task,
            running.resource,
            signal,
        )
        .expect("task should wait");

    let outcome = core
        .emit_signal(
            running.owner,
            running.owner_capability,
            running.resource,
            signal,
        )
        .expect("signal should wake waiter");

    assert_eq!(outcome.signal_event.kind, EventKind::SignalEmitted);
    assert_eq!(outcome.woken_task, Some(running.task));
    assert_eq!(
        outcome.wake_event.expect("wake event should exist").kind,
        EventKind::TaskWoken
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.run_queue()[0].task, running.task);
    assert!(!core.waiters()[0].active);
    assert_eq!(core.waiters()[0].id, waiter);
}
