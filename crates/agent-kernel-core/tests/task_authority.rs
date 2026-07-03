use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, IntentId, IntentKind, IntentStatus, KernelCore, KernelError,
    Operation, OperationSet, ResourceId, ResourceKind, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<2, 4, 4, 24, 2, 2, 2, 4, 4, 4>;

fn declare_action_intent(
    core: &mut TestCore,
    agent: AgentId,
    capability: CapabilityId,
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
fn create_task_requires_action_capability() {
    let mut core = TestCore::new();
    let agent = AgentId::new(17);
    core.register_agent(agent).expect("agent should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let observe_capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("observe capability should fit");
    let action_capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("action capability should fit");
    let intent = declare_action_intent(&mut core, agent, action_capability, resource);

    let result = core.create_task(agent, observe_capability, intent);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.tasks().len(), 0);
    assert_eq!(core.events().len(), 4);
    assert_eq!(core.events()[1].kind, EventKind::CapabilityGranted);
    assert_eq!(core.events()[2].kind, EventKind::CapabilityGranted);
    assert_eq!(core.events()[3].kind, EventKind::IntentDeclared);
}

#[test]
fn delegate_task_requires_delegate_capability_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(18);
    let assignee = AgentId::new(19);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let create_capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("create capability should fit");
    let intent = declare_action_intent(&mut core, owner, create_capability, resource);
    let task = core
        .create_task(owner, create_capability, intent)
        .expect("task should be created");
    let events_after_create = core.events().len();

    let result = core.delegate_task(owner, create_capability, task, assignee);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.events().len(), events_after_create);
}

#[test]
fn task_operations_reject_invalid_authority_and_status_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(14);
    let wrong_agent = AgentId::new(15);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(wrong_agent)
        .expect("wrong agent should register");
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
    let wrong_capability = core
        .grant_capability(
            wrong_agent,
            resource,
            OperationSet::only(Operation::Observe),
        )
        .expect("capability should fit");
    let intent = declare_action_intent(&mut core, owner, owner_capability, resource);
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should be created");
    let events_after_create = core.events().len();

    assert_eq!(
        core.delegate_task(owner, wrong_capability, task, wrong_agent),
        Err(KernelError::AgentMismatch)
    );
    assert_eq!(
        core.accept_task(wrong_agent, task),
        Err(KernelError::TaskAgentMismatch)
    );
    assert_eq!(
        core.complete_task(owner, owner_capability, task),
        Err(KernelError::TaskStatusMismatch)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.events().len(), events_after_create);
}

#[test]
fn task_store_capacity_returns_task_store_full() {
    let mut core = KernelCore::<2, 4, 4, 8, 2, 2, 2, 2, 1, 1>::new();
    let agent = AgentId::new(16);
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

    core.create_task(agent, capability, intent)
        .expect("first task should fit");
    let second_intent = core
        .declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("second intent should be declared");
    let result = core.create_task(agent, capability, second_intent);

    assert_eq!(result, Err(KernelError::TaskStoreFull));
    assert_eq!(core.tasks().len(), 1);
}

#[test]
fn cancel_task_requires_rollback_capability_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(20);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let intent = declare_action_intent(&mut core, owner, capability, resource);
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should be created");
    let events_after_create = core.events().len();

    let result = core.cancel_task(owner, capability, task);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.events().len(), events_after_create);
}

#[test]
fn cancel_task_marks_task_cancelled_and_terminal() {
    let mut core = TestCore::new();
    let owner = AgentId::new(21);
    let assignee = AgentId::new(22);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
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
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let intent = declare_action_intent(&mut core, owner, capability, resource);
    let task = core
        .create_task(owner, capability, intent)
        .expect("task should be created");

    let event = core
        .cancel_task(owner, capability, task)
        .expect("task should be cancelled");
    let events_after_cancel = core.events().len();

    assert_eq!(event.kind, EventKind::TaskCancelled);
    assert_eq!(core.tasks()[0].status, TaskStatus::Cancelled);
    assert_eq!(core.intents()[0].status, IntentStatus::Cancelled);
    assert_eq!(
        core.events()
            .last()
            .expect("intent cancel event should record")
            .kind,
        EventKind::IntentCancelled
    );
    assert_eq!(
        core.delegate_task(owner, capability, task, assignee),
        Err(KernelError::TaskStatusMismatch)
    );
    assert_eq!(core.events().len(), events_after_cancel);
}

#[test]
fn verified_task_rejects_further_transitions_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(23);
    let assignee = AgentId::new(24);
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
                .with(Operation::Verify)
                .with(Operation::Rollback),
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
    let events_after_verify = core.events().len();

    assert_eq!(
        core.verify_task(owner, owner_capability, task),
        Err(KernelError::TaskStatusMismatch)
    );
    assert_eq!(
        core.cancel_task(owner, owner_capability, task),
        Err(KernelError::TaskStatusMismatch)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Verified);
    assert_eq!(core.events().len(), events_after_verify);
}
