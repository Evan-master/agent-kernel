use agent_kernel_core::{
    AgentEntryKind, AgentId, EventKind, IntentKind, KernelCore, Operation, OperationSet,
    ResourceKind, RunQueueEntry, TaskId, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<4, 4, 8, 48, 0, 0, 0, 6, 6, 4>;

#[derive(Copy, Clone)]
struct AcceptedTask {
    task: TaskId,
}

fn accepted_task(core: &mut TestCore, owner: AgentId, assignee: AgentId) -> AcceptedTask {
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
        .expect("task should be delegated");
    let assignee_capability = core
        .tasks()
        .iter()
        .find(|task_record| task_record.id == task)
        .and_then(|task_record| task_record.delegated_capability)
        .expect("delegation should derive assignee capability");
    core.launch_task_agent(assignee, assignee_capability, task, AgentEntryKind::Worker)
        .expect("assignee should launch for delegated task");
    core.accept_task(assignee, task)
        .expect("task should be accepted");

    AcceptedTask { task }
}

#[test]
fn dispatch_next_assigns_default_quantum() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let accepted = accepted_task(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");

    let dispatched = core.dispatch_next(assignee).expect("task should dispatch");

    assert_eq!(dispatched, accepted.task);
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].run_ticks, 0);
    assert_eq!(core.tasks()[0].quantum_remaining, 1);
    let event = core.events().last().expect("dispatch should record event");
    assert_eq!(event.kind, EventKind::TaskDispatched);
    assert_eq!(event.task_quantum, Some(1));
    assert_eq!(event.task_ticks, None);
}

#[test]
fn dispatch_next_with_quantum_assigns_requested_quantum() {
    let mut core = TestCore::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let accepted = accepted_task(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");

    let dispatched = core
        .dispatch_next_with_quantum(assignee, 3)
        .expect("task should dispatch with explicit quantum");

    assert_eq!(dispatched, accepted.task);
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].quantum_remaining, 3);
    assert_eq!(core.events().last().unwrap().task_quantum, Some(3));
}

#[test]
fn tick_task_records_progress_while_quantum_remains() {
    let mut core = TestCore::new();
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
    let accepted = accepted_task(&mut core, owner, assignee);
    core.enqueue_task(assignee, accepted.task)
        .expect("accepted task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");

    let event = core
        .tick_task(assignee, accepted.task)
        .expect("running task should tick");

    assert_eq!(event.kind, EventKind::TaskTicked);
    assert_eq!(event.task, Some(accepted.task));
    assert_eq!(event.task_ticks, Some(1));
    assert_eq!(event.task_quantum, Some(1));
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].run_ticks, 1);
    assert_eq!(core.tasks()[0].quantum_remaining, 1);
    assert!(core.run_queue().is_empty());
}

#[test]
fn final_tick_expires_quantum_and_requeues_task_at_back() {
    let mut core = TestCore::new();
    let owner = AgentId::new(7);
    let first_agent = AgentId::new(8);
    let second_agent = AgentId::new(9);
    let first = accepted_task(&mut core, owner, first_agent);
    let second = accepted_task(&mut core, owner, second_agent);
    core.enqueue_task(first_agent, first.task)
        .expect("first task should enqueue");
    core.enqueue_task(second_agent, second.task)
        .expect("second task should enqueue");
    core.dispatch_next_with_quantum(first_agent, 1)
        .expect("first task should dispatch");

    let event = core
        .tick_task(first_agent, first.task)
        .expect("final tick should expire quantum");

    assert_eq!(event.kind, EventKind::TaskQuantumExpired);
    assert_eq!(event.task_ticks, Some(1));
    assert_eq!(event.task_quantum, Some(0));
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.tasks()[0].run_ticks, 1);
    assert_eq!(core.tasks()[0].quantum_remaining, 0);
    assert_eq!(
        core.run_queue(),
        &[
            RunQueueEntry {
                task: second.task,
                agent: second_agent,
            },
            RunQueueEntry {
                task: first.task,
                agent: first_agent,
            },
        ]
    );
}
