use agent_kernel_core::{
    AgentId, CapabilityId, FaultKind, IntentKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind, TaskId, TaskStatus, VerificationRequirement,
};

#[derive(Copy, Clone)]
struct PreparedTask {
    task: TaskId,
    owner_capability: CapabilityId,
}

fn accepted_task<
    const AGENTS: usize,
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
    const FAULTS: usize,
>(
    core: &mut KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        0,
        0,
        0,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        0,
        0,
        0,
        FAULTS,
    >,
    owner: AgentId,
    assignee: AgentId,
) -> PreparedTask {
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
                .with(Operation::Delegate)
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
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    core.accept_task(assignee, task)
        .expect("task should be accepted");

    PreparedTask {
        task,
        owner_capability,
    }
}

fn running_task<
    const AGENTS: usize,
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
    const FAULTS: usize,
>(
    core: &mut KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        0,
        0,
        0,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        0,
        0,
        0,
        FAULTS,
    >,
    owner: AgentId,
    assignee: AgentId,
) -> PreparedTask {
    let prepared = accepted_task(core, owner, assignee);
    core.enqueue_task(assignee, prepared.task)
        .expect("task should enqueue");
    core.dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");
    prepared
}

#[test]
fn fault_rejects_non_running_task_without_mutation() {
    let mut core = KernelCore::<2, 1, 2, 16, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1>::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let prepared = accepted_task(&mut core, owner, assignee);
    let events_before = core.events().len();

    assert_eq!(
        core.fault_task(assignee, prepared.task, FaultKind::ExecutionTrap, 1),
        Err(KernelError::TaskNotRunnable)
    );
    assert!(core.faults().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn fault_rejects_wrong_agent_without_mutation() {
    let mut core = KernelCore::<3, 1, 2, 20, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1>::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let wrong_agent = AgentId::new(5);
    let prepared = running_task(&mut core, owner, assignee);
    core.register_agent(wrong_agent)
        .expect("wrong agent should register");
    let events_before = core.events().len();

    assert_eq!(
        core.fault_task(wrong_agent, prepared.task, FaultKind::ExecutionTrap, 2),
        Err(KernelError::TaskNotRunnable)
    );
    assert!(core.faults().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn fault_store_full_leaves_running_task_unchanged() {
    let mut core = KernelCore::<2, 1, 2, 16, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0>::new();
    let owner = AgentId::new(6);
    let assignee = AgentId::new(7);
    let prepared = running_task(&mut core, owner, assignee);
    let events_before = core.events().len();

    assert_eq!(
        core.fault_task(assignee, prepared.task, FaultKind::ExecutionTrap, 3),
        Err(KernelError::FaultStoreFull)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].last_fault, None);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn fault_event_log_full_leaves_running_task_unchanged() {
    let mut core = KernelCore::<2, 1, 2, 11, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1>::new();
    let owner = AgentId::new(8);
    let assignee = AgentId::new(9);
    let prepared = running_task(&mut core, owner, assignee);

    assert_eq!(
        core.fault_task(assignee, prepared.task, FaultKind::ExecutionTrap, 4),
        Err(KernelError::EventLogFull)
    );
    assert!(core.faults().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].last_fault, None);
    assert_eq!(core.events().len(), 11);
}

#[test]
fn recover_requires_rollback_authority_without_mutation() {
    let mut core = KernelCore::<2, 1, 3, 20, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1>::new();
    let owner = AgentId::new(10);
    let assignee = AgentId::new(11);
    let prepared = running_task(&mut core, owner, assignee);
    let fault = core
        .fault_task(assignee, prepared.task, FaultKind::ExecutionTrap, 5)
        .expect("task should fault");
    let act_only = core
        .grant_capability(
            owner,
            core.tasks()[0].resource,
            OperationSet::only(Operation::Act),
        )
        .expect("act-only capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.recover_faulted_task(owner, act_only, prepared.task),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.tasks()[0].last_fault, Some(fault));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn recover_event_log_full_leaves_task_faulted() {
    let mut core = KernelCore::<2, 1, 2, 12, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1>::new();
    let owner = AgentId::new(12);
    let assignee = AgentId::new(13);
    let prepared = running_task(&mut core, owner, assignee);
    let fault = core
        .fault_task(assignee, prepared.task, FaultKind::ExecutionTrap, 6)
        .expect("fault should consume final event slot");

    assert_eq!(
        core.recover_faulted_task(owner, prepared.owner_capability, prepared.task),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.tasks()[0].last_fault, Some(fault));
    assert_eq!(core.events().len(), 12);
}
