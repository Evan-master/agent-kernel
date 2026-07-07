use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, IntentId, IntentKind,
    IntentStatus, KernelCore, KernelError, Operation, OperationSet, ResourceKind, TaskStatus,
    VerificationRequirement,
};

type TestCore = KernelCore<2, 4, 8, 64, 2, 2, 2, 4, 6, 4>;

fn grant_owner_capability<
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
    agent: AgentId,
) -> (
    agent_kernel_core::CapabilityId,
    agent_kernel_core::ResourceId,
) {
    core.register_agent(agent).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    (capability, resource)
}

fn declare_required_action_intent<
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
    agent: AgentId,
    capability: agent_kernel_core::CapabilityId,
    resource: agent_kernel_core::ResourceId,
) -> IntentId {
    core.declare_intent(
        agent,
        capability,
        resource,
        IntentKind::Act,
        VerificationRequirement::Required,
    )
    .expect("intent should be declared")
}

fn complete_task_flow<
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
    owner_capability: agent_kernel_core::CapabilityId,
    task: agent_kernel_core::TaskId,
    assignee: AgentId,
) -> agent_kernel_core::CapabilityId {
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let task_record = core
        .tasks()
        .iter()
        .find(|record| record.id == task)
        .expect("delegated task should be stored");
    let assignee_capability = task_record
        .delegated_capability
        .expect("delegation should derive capability");
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            task_record.resource,
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
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");
    core.complete_task(assignee, assignee_capability, task)
        .expect("task should complete");
    assignee_capability
}

#[test]
fn verify_task_fulfills_bound_intent_and_records_event() {
    let mut core = TestCore::new();
    let owner = AgentId::new(10);
    let assignee = AgentId::new(11);
    let (owner_capability, resource) = grant_owner_capability(&mut core, owner);
    core.register_agent(assignee)
        .expect("assignee should register");
    let intent = declare_required_action_intent(&mut core, owner, owner_capability, resource);
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");
    complete_task_flow(&mut core, owner, owner_capability, task, assignee);

    core.verify_task(owner, owner_capability, task)
        .expect("task should verify");

    assert_eq!(core.tasks()[0].status, TaskStatus::Verified);
    assert_eq!(core.intents()[0].status, IntentStatus::Fulfilled);
    let event = core.events().last().expect("event should be recorded");
    assert_eq!(event.kind, EventKind::IntentFulfilled);
    assert_eq!(event.intent, Some(intent));
    assert_eq!(event.task, Some(task));
    assert_eq!(event.verification, VerificationRequirement::Required);
}

#[test]
fn cancel_task_cancels_bound_intent_and_records_event() {
    let mut core = TestCore::new();
    let owner = AgentId::new(12);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let intent = declare_required_action_intent(&mut core, owner, capability, resource);
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should be created");

    core.cancel_task(owner, capability, task)
        .expect("task should cancel");

    assert_eq!(core.tasks()[0].status, TaskStatus::Cancelled);
    assert_eq!(core.intents()[0].status, IntentStatus::Cancelled);
    let event = core.events().last().expect("event should be recorded");
    assert_eq!(event.kind, EventKind::IntentCancelled);
    assert_eq!(event.intent, Some(intent));
    assert_eq!(event.task, Some(task));
}

#[test]
fn verify_task_requires_two_event_slots_without_mutation() {
    let mut core = KernelCore::<2, 1, 4, 15, 2, 2, 2, 1, 1, 1>::new();
    let owner = AgentId::new(13);
    let assignee = AgentId::new(14);
    let (owner_capability, resource) = grant_owner_capability(&mut core, owner);
    core.register_agent(assignee)
        .expect("assignee should register");
    let intent = declare_required_action_intent(&mut core, owner, owner_capability, resource);
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");
    complete_task_flow(&mut core, owner, owner_capability, task, assignee);
    assert_eq!(core.events().len(), 15);

    let result = core.verify_task(owner, owner_capability, task);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
    assert_eq!(core.intents()[0].status, IntentStatus::Bound);
    assert_eq!(core.events().len(), 15);
}

#[test]
fn cancel_task_requires_two_event_slots_without_mutation() {
    let mut core = KernelCore::<2, 1, 1, 5, 2, 2, 2, 1, 1, 0>::new();
    let owner = AgentId::new(15);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let intent = declare_required_action_intent(&mut core, owner, capability, resource);
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should be created");
    assert_eq!(core.events().len(), 5);

    let result = core.cancel_task(owner, capability, task);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.intents()[0].status, IntentStatus::Bound);
    assert_eq!(core.events().len(), 5);
}

#[test]
fn task_lifecycle_events_carry_task_intent() {
    let mut core = TestCore::new();
    let owner = AgentId::new(16);
    let assignee = AgentId::new(17);
    let (owner_capability, resource) = grant_owner_capability(&mut core, owner);
    core.register_agent(assignee)
        .expect("assignee should register");
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
    complete_task_flow(&mut core, owner, owner_capability, task, assignee);
    core.verify_task(owner, owner_capability, task)
        .expect("task should verify");

    for event in &core.events()[3..=15] {
        if matches!(
            event.kind,
            EventKind::AgentImageRegistered | EventKind::AgentImageVerified
        ) {
            continue;
        }
        assert_eq!(event.intent, Some(intent));
    }
    assert_eq!(core.events()[5].kind, EventKind::IntentBound);
    assert_eq!(core.events()[16].kind, EventKind::IntentFulfilled);
}
