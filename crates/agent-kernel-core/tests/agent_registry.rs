use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, AgentStatus, CapabilityId,
    EventKind, IntentKind, KernelCore, KernelError, Operation, OperationSet, ResourceKind,
    TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<2, 1, 1, 8, 0, 0, 0, 0, 0, 0>;

#[test]
fn register_agent_records_agent_and_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(1);

    let event = core
        .register_agent(agent)
        .expect("agent registration should fit");

    assert_eq!(core.agents().len(), 1);
    assert_eq!(core.agents()[0].id, agent);
    assert_eq!(core.agents()[0].status, AgentStatus::Active);
    assert_eq!(event.kind, EventKind::AgentRegistered);
    assert_eq!(event.agent, agent);
    assert_eq!(event.target_agent, Some(agent));
    assert_eq!(event.resource, None);
    assert_eq!(event.capability, None);
    assert_eq!(core.events().len(), 1);
    assert_eq!(core.events()[0], event);
}

#[test]
fn register_agent_rejects_duplicate_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(2);
    core.register_agent(agent)
        .expect("first registration should fit");
    let events_after_registration = core.events().len();

    let result = core.register_agent(agent);

    assert_eq!(result, Err(KernelError::AgentAlreadyExists));
    assert_eq!(core.agents().len(), 1);
    assert_eq!(core.agents()[0].status, AgentStatus::Active);
    assert_eq!(core.events().len(), events_after_registration);
}

#[test]
fn register_agent_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<0, 1, 1, 4, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(3);

    let result = core.register_agent(agent);

    assert_eq!(result, Err(KernelError::AgentStoreFull));
    assert!(core.agents().is_empty());
    assert!(core.events().is_empty());
}

#[test]
fn register_agent_event_log_full_leaves_agents_unchanged() {
    let mut core = KernelCore::<1, 1, 1, 0, 0, 0, 0, 0, 0, 0>::new();
    let agent = AgentId::new(4);

    let result = core.register_agent(agent);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert!(core.agents().is_empty());
    assert!(core.events().is_empty());
}

#[test]
fn grant_capability_rejects_unregistered_agent_without_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(5);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");

    let result = core.grant_capability(agent, resource, OperationSet::only(Operation::Observe));

    assert_eq!(result, Err(KernelError::AgentNotFound));
    assert!(core.events().is_empty());
    core.register_agent(agent)
        .expect("agent registration should fit after failed grant");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("grant should fit after registration");
    assert_eq!(capability, CapabilityId::new(1));
}

#[test]
fn delegate_task_rejects_unregistered_target_agent_without_mutation() {
    let mut core = KernelCore::<2, 1, 2, 8, 1, 1, 1, 1, 1, 1>::new();
    let owner = AgentId::new(6);
    let target = AgentId::new(7);
    core.register_agent(owner)
        .expect("owner registration should fit");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
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
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should fit");
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should fit");
    let events_after_create = core.events().len();

    let result = core.delegate_task(owner, capability, task, target);

    assert_eq!(result, Err(KernelError::AgentNotFound));
    assert_eq!(core.events().len(), events_after_create);
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.tasks()[0].assignee, None);
    assert_eq!(core.tasks()[0].delegated_capability, None);
}

#[test]
fn observe_rejects_unregistered_actor_before_capability_mismatch_without_event() {
    let mut core = KernelCore::<1, 1, 1, 4, 0, 1, 0, 0, 0, 0>::new();
    let owner = AgentId::new(8);
    let intruder = AgentId::new(9);
    core.register_agent(owner)
        .expect("owner registration should fit");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Observe))
        .expect("owner capability should fit");
    let events_after_grant = core.events().len();

    let result = core.observe(intruder, capability, resource);

    assert_eq!(result, Err(KernelError::AgentNotFound));
    assert!(core.observations().is_empty());
    assert_eq!(core.events().len(), events_after_grant);
}

#[test]
fn accept_task_rejects_unregistered_actor_without_mutation() {
    let mut core = KernelCore::<2, 1, 2, 8, 1, 1, 1, 1, 1, 1>::new();
    let owner = AgentId::new(10);
    let assignee = AgentId::new(11);
    let intruder = AgentId::new(12);
    core.register_agent(owner)
        .expect("owner registration should fit");
    core.register_agent(assignee)
        .expect("assignee registration should fit");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
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
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should fit");
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should fit");
    core.delegate_task(owner, capability, task, assignee)
        .expect("task should delegate");
    let events_after_delegation = core.events().len();

    let result = core.accept_task(intruder, task);

    assert_eq!(result, Err(KernelError::AgentNotFound));
    assert_eq!(core.tasks()[0].status, TaskStatus::Delegated);
    assert_eq!(core.events().len(), events_after_delegation);
}

#[test]
fn dispatch_next_rejects_unregistered_actor_without_mutation() {
    let mut core = KernelCore::<2, 1, 2, 13, 1, 1, 1, 1, 1, 1>::new();
    let owner = AgentId::new(13);
    let runner = AgentId::new(14);
    let intruder = AgentId::new(15);
    core.register_agent(owner)
        .expect("owner registration should fit");
    core.register_agent(runner)
        .expect("runner registration should fit");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
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
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should fit");
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should fit");
    core.delegate_task(owner, capability, task, runner)
        .expect("task should delegate");
    let runner_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    core.verify_agent_image(owner, capability, image)
        .expect("image should verify");
    core.launch_task_agent(
        runner,
        runner_capability,
        task,
        image,
        AgentEntryKind::Worker,
    )
    .expect("runner should launch for delegated task");
    core.accept_task(runner, task)
        .expect("task should be accepted");
    core.enqueue_task(runner, task)
        .expect("task should be queued");
    let events_after_enqueue = core.events().len();

    let result = core.dispatch_next(intruder);

    assert_eq!(result, Err(KernelError::AgentNotFound));
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.run_queue().len(), 1);
    assert_eq!(core.events().len(), events_after_enqueue);
}
