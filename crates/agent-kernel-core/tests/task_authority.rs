use agent_kernel_core::{
    AgentId, EventKind, KernelCore, KernelError, Operation, OperationSet, ResourceKind, TaskStatus,
};

type TestCore = KernelCore<4, 4, 16, 4, 4>;

#[test]
fn create_task_requires_action_capability() {
    let mut core = TestCore::new();
    let agent = AgentId::new(17);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let result = core.create_task(agent, capability, resource);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.tasks().len(), 0);
    assert_eq!(core.events().len(), 0);
}

#[test]
fn delegate_task_requires_delegate_capability_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(18);
    let assignee = AgentId::new(19);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let create_capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("create capability should fit");
    let task = core
        .create_task(owner, create_capability, resource)
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
    let task = core
        .create_task(owner, owner_capability, resource)
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
    let mut core = KernelCore::<4, 4, 8, 1, 1>::new();
    let agent = AgentId::new(16);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");

    core.create_task(agent, capability, resource)
        .expect("first task should fit");
    let result = core.create_task(agent, capability, resource);

    assert_eq!(result, Err(KernelError::TaskStoreFull));
    assert_eq!(core.tasks().len(), 1);
}

#[test]
fn cancel_task_requires_rollback_capability_without_events() {
    let mut core = TestCore::new();
    let owner = AgentId::new(20);
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let task = core
        .create_task(owner, capability, resource)
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
    let task = core
        .create_task(owner, capability, resource)
        .expect("task should be created");

    let event = core
        .cancel_task(owner, capability, task)
        .expect("task should be cancelled");
    let events_after_cancel = core.events().len();

    assert_eq!(event.kind, EventKind::TaskCancelled);
    assert_eq!(core.tasks()[0].status, TaskStatus::Cancelled);
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
    let assignee_capability = core
        .grant_capability(assignee, resource, OperationSet::only(Operation::Act))
        .expect("assignee capability should fit");
    let task = core
        .create_task(owner, owner_capability, resource)
        .expect("task should be created");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    core.accept_task(assignee, task)
        .expect("task should be accepted");
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
