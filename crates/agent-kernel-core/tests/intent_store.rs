use agent_kernel_core::{
    AgentId, EventKind, IntentId, IntentKind, IntentStatus, KernelCore, KernelError, Operation,
    OperationSet, ResourceKind, TaskStatus, VerificationRequirement,
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

#[test]
fn declare_intent_records_typed_intent() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");

    let intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");

    assert_eq!(intent, IntentId::new(1));
    assert_eq!(core.intents().len(), 1);
    assert_eq!(core.intents()[0].id, intent);
    assert_eq!(core.intents()[0].owner, agent);
    assert_eq!(core.intents()[0].resource, resource);
    assert_eq!(core.intents()[0].kind, IntentKind::Act);
    assert_eq!(core.intents()[0].status, IntentStatus::Declared);
    assert_eq!(
        core.intents()[0].verification,
        VerificationRequirement::Required
    );
    assert_eq!(core.events()[2].kind, EventKind::IntentDeclared);
    assert_eq!(core.events()[2].intent, Some(intent));
    assert_eq!(core.events()[2].intent_kind, Some(IntentKind::Act));
    assert_eq!(
        core.events()[2].verification,
        VerificationRequirement::Required
    );
    assert_eq!(core.events()[2].operation, Some(Operation::Act));
}

#[test]
fn declare_intent_requires_matching_operation_capability() {
    let mut core = TestCore::new();
    let agent = AgentId::new(2);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");
    let events_after_grant = core.events().len();

    let result = core.declare_intent(
        agent,
        capability,
        resource,
        IntentKind::Act,
        VerificationRequirement::Required,
    );

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert!(core.intents().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
}

#[test]
fn declare_intent_returns_intent_store_full_without_mutation() {
    let mut core = KernelCore::<2, 1, 1, 4, 2, 2, 2, 0, 0, 0>::new();
    let agent = AgentId::new(3);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let events_after_grant = core.events().len();

    let result = core.declare_intent(
        agent,
        capability,
        resource,
        IntentKind::Act,
        VerificationRequirement::Required,
    );

    assert_eq!(result, Err(KernelError::IntentStoreFull));
    assert!(core.intents().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
}

#[test]
fn declare_intent_returns_event_log_full_without_mutation() {
    let mut core = KernelCore::<2, 1, 1, 2, 2, 2, 2, 1, 0, 0>::new();
    let agent = AgentId::new(4);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("grant should consume only event slot");

    let result = core.declare_intent(
        agent,
        capability,
        resource,
        IntentKind::Act,
        VerificationRequirement::Required,
    );

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.intents().is_empty());
    assert_eq!(core.events().len(), 2);
}

#[test]
fn create_task_from_intent_binds_task_and_event_to_intent() {
    let mut core = TestCore::new();
    let agent = AgentId::new(5);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");

    let task = core
        .create_task(agent, capability, intent)
        .expect("task should be created from intent");

    assert_eq!(core.tasks()[0].id, task);
    assert_eq!(core.tasks()[0].intent, intent);
    assert_eq!(core.tasks()[0].resource, resource);
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.intents()[0].status, IntentStatus::Bound);
    assert_eq!(core.events()[3].kind, EventKind::TaskCreated);
    assert_eq!(core.events()[3].intent, Some(intent));
    assert_eq!(core.events()[3].task, Some(task));
    assert_eq!(core.events()[4].kind, EventKind::IntentBound);
    assert_eq!(core.events()[4].intent, Some(intent));
    assert_eq!(core.events()[4].task, Some(task));
    assert_eq!(
        core.events()[4].verification,
        VerificationRequirement::Required
    );
}

#[test]
fn create_task_rejects_other_agents_intent_without_mutation() {
    let mut core = TestCore::new();
    let owner = AgentId::new(6);
    let other = AgentId::new(7);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(other).expect("other should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("owner capability should fit");
    let other_capability = core
        .grant_capability(other, resource, OperationSet::only(Operation::Act))
        .expect("other capability should fit");
    let intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let events_after_intent = core.events().len();

    let result = core.create_task(other, other_capability, intent);

    assert_eq!(result, Err(KernelError::IntentAgentMismatch));
    assert!(core.tasks().is_empty());
    assert_eq!(core.events().len(), events_after_intent);
}

#[test]
fn create_task_rejects_already_bound_intent_without_mutation() {
    let mut core = TestCore::new();
    let agent = AgentId::new(8);
    let (capability, resource) = grant_owner_capability(&mut core, agent);
    let intent = declare_required_action_intent(&mut core, agent, capability, resource);
    let first = core
        .create_task(agent, capability, intent)
        .expect("first task should bind intent");
    let events_after_first_task = core.events().len();

    let result = core.create_task(agent, capability, intent);

    assert_eq!(result, Err(KernelError::IntentStatusMismatch));
    assert_eq!(core.tasks().len(), 1);
    assert_eq!(core.tasks()[0].id, first);
    assert_eq!(core.intents()[0].status, IntentStatus::Bound);
    assert_eq!(core.events().len(), events_after_first_task);
}

#[test]
fn create_task_requires_two_event_slots_without_mutation() {
    let mut core = KernelCore::<2, 1, 1, 4, 2, 2, 2, 1, 1, 0>::new();
    let agent = AgentId::new(9);
    let (capability, resource) = grant_owner_capability(&mut core, agent);
    let intent = declare_required_action_intent(&mut core, agent, capability, resource);
    assert_eq!(core.events().len(), 3);

    let result = core.create_task(agent, capability, intent);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.tasks().is_empty());
    assert_eq!(core.intents()[0].status, IntentStatus::Declared);
    assert_eq!(core.events().len(), 3);
}
