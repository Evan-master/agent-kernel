use agent_kernel_core::{
    ActionId, AgentId, CapabilityId, EventKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceId, ResourceKind, TaskId, TaskStatus,
};

type TestCore = KernelCore<4, 8, 32, 6, 4>;

#[derive(Copy, Clone)]
struct DelegatedTask {
    task: TaskId,
    resource: ResourceId,
    owner_capability: CapabilityId,
    delegated_capability: CapabilityId,
}

fn create_delegated_task(core: &mut TestCore, owner: AgentId, assignee: AgentId) -> DelegatedTask {
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
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");
    let event = core
        .delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    let delegated_capability = event
        .capability
        .expect("delegation should expose derived capability");

    DelegatedTask {
        task,
        resource,
        owner_capability,
        delegated_capability,
    }
}

fn dispatch_task(core: &mut TestCore, assignee: AgentId, task: TaskId) {
    core.accept_task(assignee, task)
        .expect("task should be accepted");
    core.enqueue_task(assignee, task)
        .expect("task should enqueue");
    core.dispatch_next(assignee).expect("task should dispatch");
}

#[test]
fn delegate_task_derives_task_scoped_capability_for_assignee() {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let delegated = create_delegated_task(&mut core, owner, assignee);

    assert_ne!(delegated.delegated_capability, delegated.owner_capability);
    assert_eq!(
        core.tasks()[0].delegated_capability,
        Some(delegated.delegated_capability)
    );
    assert_eq!(core.tasks()[0].assignee, Some(assignee));
    assert_eq!(core.tasks()[0].status, TaskStatus::Delegated);
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::DelegationRequested
    );
    assert_eq!(
        core.events().last().unwrap().capability,
        Some(delegated.delegated_capability)
    );
}

#[test]
fn derived_capability_completes_dispatched_task_without_manual_grant() {
    let mut core = TestCore::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    let delegated = create_delegated_task(&mut core, owner, assignee);
    dispatch_task(&mut core, assignee, delegated.task);

    let event = core
        .complete_task(assignee, delegated.delegated_capability, delegated.task)
        .expect("derived capability should complete assigned running task");

    assert_eq!(event.kind, EventKind::TaskCompleted);
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
}

#[test]
fn derived_capability_cannot_authorize_generic_action() {
    let mut core = TestCore::new();
    let owner = AgentId::new(5);
    let assignee = AgentId::new(6);
    let delegated = create_delegated_task(&mut core, owner, assignee);
    let events_before = core.events().len();

    let result = core.act(
        assignee,
        delegated.delegated_capability,
        ActionId::new(7),
        delegated.resource,
    );

    assert_eq!(result, Err(KernelError::CapabilityScopeMismatch));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn derived_capability_cannot_complete_a_different_task() {
    let mut core = TestCore::new();
    let owner = AgentId::new(7);
    let assignee = AgentId::new(8);
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
    let first = core
        .create_task(owner, owner_capability, resource)
        .expect("first task should be created");
    let first_event = core
        .delegate_task(owner, owner_capability, first, assignee)
        .expect("first task should delegate");
    let first_capability = first_event
        .capability
        .expect("first delegation should derive capability");
    let second = core
        .create_task(owner, owner_capability, resource)
        .expect("second task should be created");
    core.delegate_task(owner, owner_capability, second, assignee)
        .expect("second task should delegate");
    dispatch_task(&mut core, assignee, second);
    let events_before = core.events().len();

    let result = core.complete_task(assignee, first_capability, second);

    assert_eq!(result, Err(KernelError::CapabilityScopeMismatch));
    assert_eq!(core.tasks()[1].status, TaskStatus::Running);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn delegate_requires_source_act_authority_for_derived_capability() {
    let mut core = TestCore::new();
    let owner = AgentId::new(9);
    let assignee = AgentId::new(10);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let create_capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("create capability should fit");
    let delegate_only_capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Delegate))
        .expect("delegate capability should fit");
    let task = core
        .create_task(owner, create_capability, resource)
        .expect("task should be created");
    let events_after_create = core.events().len();

    let result = core.delegate_task(owner, delegate_only_capability, task, assignee);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.tasks()[0].delegated_capability, None);
    assert_eq!(core.events().len(), events_after_create);
}

#[test]
fn delegate_returns_capability_store_full_without_state_changes() {
    let mut core = KernelCore::<2, 1, 8, 2, 2>::new();
    let owner = AgentId::new(11);
    let assignee = AgentId::new(12);
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
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");
    let events_after_create = core.events().len();

    let result = core.delegate_task(owner, owner_capability, task, assignee);

    assert_eq!(result, Err(KernelError::CapabilityStoreFull));
    assert_eq!(core.tasks()[0].status, TaskStatus::Created);
    assert_eq!(core.tasks()[0].assignee, None);
    assert_eq!(core.tasks()[0].delegated_capability, None);
    assert_eq!(core.events().len(), events_after_create);
}
