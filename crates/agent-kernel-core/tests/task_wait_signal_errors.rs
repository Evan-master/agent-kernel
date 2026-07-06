use agent_kernel_core::{
    AgentEntryKind, AgentId, CapabilityId, EventKind, IntentKind, KernelCore, KernelError,
    Operation, OperationSet, ResourceId, ResourceKind, SignalKey, TaskId, TaskStatus,
    VerificationRequirement,
};

type SignalCore<
    const EVENTS: usize,
    const RUN_QUEUE: usize,
    const WAITERS: usize,
    const CAPS: usize = 2,
> = KernelCore<2, 1, CAPS, EVENTS, 0, 0, 0, 1, 1, RUN_QUEUE, 0, 0, 0, 0, 0, 0, WAITERS>;

#[derive(Copy, Clone)]
struct PreparedTask {
    owner: AgentId,
    assignee: AgentId,
    owner_capability: CapabilityId,
    assignee_capability: CapabilityId,
    resource: ResourceId,
    task: TaskId,
}

fn prepared_task<
    const EVENTS: usize,
    const RUN_QUEUE: usize,
    const WAITERS: usize,
    const CAPS: usize,
>(
    core: &mut SignalCore<EVENTS, RUN_QUEUE, WAITERS, CAPS>,
    dispatch: bool,
) -> PreparedTask {
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
        .expect("intent should declare");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should create");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive task capability");
    core.launch_task_agent(assignee, assignee_capability, task, AgentEntryKind::Worker)
        .expect("assignee should launch for delegated task");
    core.accept_task(assignee, task)
        .expect("task should accept");
    if dispatch {
        core.enqueue_task(assignee, task)
            .expect("task should enqueue");
        core.dispatch_next_with_quantum(assignee, 2)
            .expect("task should dispatch");
    }
    PreparedTask {
        owner,
        assignee,
        owner_capability,
        assignee_capability,
        resource,
        task,
    }
}

#[test]
fn wait_task_requires_running_task_without_mutation() {
    let mut core = SignalCore::<14, 1, 1>::new();
    let prepared = prepared_task(&mut core, false);
    let events_before = core.events().len();

    assert_eq!(
        core.wait_task(
            prepared.assignee,
            prepared.assignee_capability,
            prepared.task,
            prepared.resource,
            SignalKey::new(1),
        ),
        Err(KernelError::TaskStatusMismatch)
    );
    assert!(core.waiters().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn wait_task_store_full_leaves_running_task() {
    let mut core = SignalCore::<16, 1, 0>::new();
    let prepared = prepared_task(&mut core, true);
    let events_before = core.events().len();

    assert_eq!(
        core.wait_task(
            prepared.assignee,
            prepared.assignee_capability,
            prepared.task,
            prepared.resource,
            SignalKey::new(2),
        ),
        Err(KernelError::WaiterStoreFull)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn emit_signal_requires_act_authority_without_mutation() {
    let mut core = SignalCore::<18, 1, 1, 3>::new();
    let prepared = prepared_task(&mut core, true);
    core.wait_task(
        prepared.assignee,
        prepared.assignee_capability,
        prepared.task,
        prepared.resource,
        SignalKey::new(3),
    )
    .expect("task should wait");
    let observer_capability = core
        .grant_capability(
            prepared.owner,
            prepared.resource,
            OperationSet::only(Operation::Observe),
        )
        .expect("observer capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.emit_signal(
            prepared.owner,
            observer_capability,
            prepared.resource,
            SignalKey::new(3),
        ),
        Err(KernelError::OperationDenied)
    );
    assert!(core.waiters()[0].active);
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn emit_signal_event_log_full_leaves_waiter_waiting() {
    let mut core = SignalCore::<13, 1, 1>::new();
    let prepared = prepared_task(&mut core, true);
    core.wait_task(
        prepared.assignee,
        prepared.assignee_capability,
        prepared.task,
        prepared.resource,
        SignalKey::new(5),
    )
    .expect("task should wait");

    assert_eq!(
        core.emit_signal(
            prepared.owner,
            prepared.owner_capability,
            prepared.resource,
            SignalKey::new(5),
        ),
        Err(KernelError::EventLogFull)
    );
    assert!(core.waiters()[0].active);
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.events().last().unwrap().kind, EventKind::TaskWaiting);
}
