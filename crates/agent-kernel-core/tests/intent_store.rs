use agent_kernel_core::{
    AgentId, EventKind, IntentId, IntentKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<4, 8, 64, 4, 6, 4>;

fn grant_owner_capability(core: &mut TestCore, agent: AgentId) -> agent_kernel_core::CapabilityId {
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    core.grant_capability(
        agent,
        resource,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Delegate)
            .with(Operation::Verify)
            .with(Operation::Rollback),
    )
    .expect("capability should fit")
}

#[test]
fn declare_intent_records_typed_intent() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);
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
    assert_eq!(
        core.intents()[0].verification,
        VerificationRequirement::Required
    );
    assert_eq!(core.events()[1].kind, EventKind::IntentDeclared);
    assert_eq!(core.events()[1].intent, Some(intent));
    assert_eq!(core.events()[1].intent_kind, Some(IntentKind::Act));
    assert_eq!(
        core.events()[1].verification,
        VerificationRequirement::Required
    );
    assert_eq!(core.events()[1].operation, Some(Operation::Act));
}

#[test]
fn declare_intent_requires_matching_operation_capability() {
    let mut core = TestCore::new();
    let agent = AgentId::new(2);
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
    let mut core = KernelCore::<1, 1, 4, 0, 0, 0>::new();
    let agent = AgentId::new(3);
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
    let mut core = KernelCore::<1, 1, 1, 1, 0, 0>::new();
    let agent = AgentId::new(4);
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
    assert_eq!(core.events().len(), 1);
}

#[test]
fn create_task_from_intent_binds_task_and_event_to_intent() {
    let mut core = TestCore::new();
    let agent = AgentId::new(5);
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
    assert_eq!(core.events()[2].kind, EventKind::TaskCreated);
    assert_eq!(core.events()[2].intent, Some(intent));
    assert_eq!(core.events()[2].task, Some(task));
}

#[test]
fn create_task_rejects_other_agents_intent_without_mutation() {
    let mut core = TestCore::new();
    let owner = AgentId::new(6);
    let other = AgentId::new(7);
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
fn task_lifecycle_events_carry_task_intent() {
    let mut core = TestCore::new();
    let owner = AgentId::new(8);
    let assignee = AgentId::new(9);
    let owner_capability = grant_owner_capability(&mut core, owner);
    let resource = core.events()[0]
        .resource
        .expect("grant event should identify resource");
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
        .expect("delegation should derive capability");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");
    core.complete_task(assignee, assignee_capability, task)
        .expect("task should complete");
    core.verify_task(owner, owner_capability, task)
        .expect("task should verify");

    for event in &core.events()[2..=9] {
        assert_eq!(event.intent, Some(intent));
    }
}
