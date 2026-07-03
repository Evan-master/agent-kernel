use agent_kernel_core::{
    AgentId, EventKind, IntentId, IntentKind, KernelCore, Operation, OperationSet, ResourceId,
    ResourceKind, TaskId, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<4, 4, 16, 2, 2, 4, 4, 4>;

fn declare_action_intent(
    core: &mut TestCore,
    agent: AgentId,
    capability: agent_kernel_core::CapabilityId,
    resource: ResourceId,
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
fn create_task_allocates_kernel_task_and_records_event() {
    let mut core = TestCore::new();
    let agent = AgentId::new(11);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let intent = declare_action_intent(&mut core, agent, capability, resource);

    let task = core
        .create_task(agent, capability, intent)
        .expect("task should be created");

    assert_eq!(task, TaskId::new(1));
    assert_eq!(core.tasks().len(), 1);
    assert_eq!(core.tasks()[0].id, task);
    assert_eq!(core.tasks()[0].intent, intent);
    assert_eq!(core.tasks()[0].owner, agent);
    assert_eq!(core.tasks()[0].resource, resource);
    assert_eq!(core.tasks()[0].assignee, None);
    assert_eq!(core.tasks()[0].delegated_capability, None);
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.events()[0].kind, EventKind::CapabilityGranted);
    assert_eq!(core.events()[1].kind, EventKind::IntentDeclared);
    assert_eq!(core.events()[2].kind, EventKind::TaskCreated);
    assert_eq!(core.events()[2].task, Some(task));
    assert_eq!(core.events()[2].intent, Some(intent));
    assert_eq!(core.events()[3].kind, EventKind::IntentBound);
    assert_eq!(core.events()[3].task, Some(task));
    assert_eq!(core.events()[3].intent, Some(intent));
}

#[test]
fn task_lifecycle_reaches_verified_through_authorized_transitions() {
    let mut core = TestCore::new();
    let owner = AgentId::new(12);
    let assignee = AgentId::new(13);
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
    let intent = declare_action_intent(&mut core, owner, owner_capability, resource);
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive assignee capability");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");
    core.complete_task(assignee, assignee_capability, task)
        .expect("task should be completed");
    core.verify_task(owner, owner_capability, task)
        .expect("task should be verified");

    assert_eq!(core.tasks()[0].status, TaskStatus::Verified);
    assert_eq!(core.tasks()[0].assignee, Some(assignee));
    assert_eq!(core.events()[0].kind, EventKind::CapabilityGranted);
    assert_eq!(core.events()[1].kind, EventKind::IntentDeclared);
    assert_eq!(core.events()[2].kind, EventKind::TaskCreated);
    assert_eq!(core.events()[3].kind, EventKind::IntentBound);
    assert_eq!(core.events()[4].kind, EventKind::CapabilityDerived);
    assert_eq!(core.events()[4].target_agent, Some(assignee));
    assert_eq!(core.events()[5].kind, EventKind::DelegationRequested);
    assert_eq!(core.events()[5].target_agent, Some(assignee));
    assert_eq!(core.events()[6].kind, EventKind::TaskAccepted);
    assert_eq!(core.events()[7].kind, EventKind::TaskQueued);
    assert_eq!(core.events()[8].kind, EventKind::TaskDispatched);
    assert_eq!(core.events()[9].kind, EventKind::TaskCompleted);
    assert_eq!(core.events()[10].kind, EventKind::TaskVerified);
    assert_eq!(core.events()[11].kind, EventKind::IntentFulfilled);
    for event in &core.events()[2..=11] {
        assert_eq!(event.intent, Some(intent));
    }
}
