use agent_kernel_core::{
    AgentId, CapabilityId, IntentId, IntentKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceId, ResourceKind, TaskId, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<2, 4, 8, 32, 2, 2, 2, 6, 6, 4>;

#[derive(Copy, Clone)]
struct RunningDelegatedTask {
    task: TaskId,
    source_capability: CapabilityId,
    delegated_capability: CapabilityId,
}

fn grant_source_capability(
    core: &mut TestCore,
    owner: AgentId,
    resource: ResourceId,
) -> CapabilityId {
    core.grant_capability(
        owner,
        resource,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Delegate)
            .with(Operation::Verify)
            .with(Operation::Rollback),
    )
    .expect("source capability should fit")
}

fn declare_action_intent(
    core: &mut TestCore,
    owner: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
) -> IntentId {
    core.declare_intent(
        owner,
        capability,
        resource,
        IntentKind::Act,
        VerificationRequirement::Required,
    )
    .expect("intent should be declared")
}

fn running_delegated_task(
    core: &mut TestCore,
    owner: AgentId,
    assignee: AgentId,
) -> RunningDelegatedTask {
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source_capability = grant_source_capability(core, owner, resource);
    let intent = declare_action_intent(core, owner, source_capability, resource);
    let task = core
        .create_task(owner, source_capability, intent)
        .expect("task should be created");
    let event = core
        .delegate_task(owner, source_capability, task, assignee)
        .expect("task should be delegated");
    let delegated_capability = event
        .capability
        .expect("delegation should expose derived capability");

    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");

    RunningDelegatedTask {
        task,
        source_capability,
        delegated_capability,
    }
}

#[test]
fn revoking_source_capability_invalidates_derived_task_capability() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let delegated = running_delegated_task(&mut core, owner, assignee);

    core.revoke_capability(delegated.source_capability)
        .expect("source capability should revoke");
    let events_before = core.events().len();

    let result = core.complete_task(assignee, delegated.delegated_capability, delegated.task);

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn revoking_derived_capability_rejects_task_completion() {
    let mut core = TestCore::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let delegated = running_delegated_task(&mut core, owner, assignee);

    core.revoke_capability(delegated.delegated_capability)
        .expect("derived capability should revoke");
    let events_before = core.events().len();

    let result = core.complete_task(assignee, delegated.delegated_capability, delegated.task);

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn revoking_unrelated_capability_does_not_invalidate_derived_task_capability() {
    let mut core = TestCore::new();
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
    let delegated = running_delegated_task(&mut core, owner, assignee);
    let unrelated_resource = core
        .register_resource(ResourceKind::Device, None)
        .expect("unrelated resource should fit");
    let unrelated = core
        .grant_capability(
            owner,
            unrelated_resource,
            OperationSet::only(Operation::Act),
        )
        .expect("unrelated capability should fit");

    core.revoke_capability(unrelated)
        .expect("unrelated capability should revoke");

    core.complete_task(assignee, delegated.delegated_capability, delegated.task)
        .expect("unrelated revocation should not affect derived task capability");

    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
}

#[test]
fn revoking_one_source_invalidates_multiple_derived_capabilities() {
    let mut core = TestCore::new();
    let owner = AgentId::new(7);
    let assignee = AgentId::new(8);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(assignee)
        .expect("assignee should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source_capability = grant_source_capability(&mut core, owner, resource);
    let first_intent = declare_action_intent(&mut core, owner, source_capability, resource);
    let first = core
        .create_task(owner, source_capability, first_intent)
        .expect("first task should be created");
    let first_capability = core
        .delegate_task(owner, source_capability, first, assignee)
        .expect("first task should delegate")
        .capability
        .expect("first delegation should derive capability");
    let second_intent = declare_action_intent(&mut core, owner, source_capability, resource);
    let second = core
        .create_task(owner, source_capability, second_intent)
        .expect("second task should be created");
    let second_capability = core
        .delegate_task(owner, source_capability, second, assignee)
        .expect("second task should delegate")
        .capability
        .expect("second delegation should derive capability");

    core.accept_task(assignee, first)
        .expect("first task should accept");
    core.enqueue_task(assignee, first)
        .expect("first task should enqueue");
    core.dispatch_next(assignee)
        .expect("first task should dispatch");
    core.accept_task(assignee, second)
        .expect("second task should accept");
    core.enqueue_task(assignee, second)
        .expect("second task should enqueue");
    core.dispatch_next(assignee)
        .expect("second task should dispatch");

    core.revoke_capability(source_capability)
        .expect("source capability should revoke");
    let events_before = core.events().len();

    assert_eq!(
        core.complete_task(assignee, first_capability, first),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(
        core.complete_task(assignee, second_capability, second),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[1].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}
