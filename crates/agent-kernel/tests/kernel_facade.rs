use agent_kernel::AgentKernel;
use agent_kernel_core::{
    ActionId, AgentId, CheckpointId, EventKind, Operation, OperationSet, ResourceKind,
    RunQueueEntry, TaskId, TaskStatus,
};

type TestKernel = AgentKernel<4, 6, 32, 6, 4>;

#[test]
fn kernel_starts_with_empty_event_log() {
    let kernel = TestKernel::new();

    assert!(kernel.events().is_empty());
    assert!(kernel.tasks().is_empty());
}

#[test]
fn observe_syscall_records_observation_event() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(42);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(agent, resource, OperationSet::only(Operation::Observe))
        .expect("capability should fit");

    let event = kernel
        .sys_observe(agent, capability, resource)
        .expect("observe should be authorized");

    assert_eq!(event.kind, EventKind::Observation);
    assert_eq!(event.agent, agent);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(kernel.events().len(), 1);
}

#[test]
fn checkpoint_and_rollback_syscalls_record_kernel_events() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(77);
    let checkpoint = CheckpointId::new(5);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Checkpoint)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");

    kernel
        .sys_checkpoint(agent, capability, checkpoint, resource)
        .expect("checkpoint event should fit");
    kernel
        .sys_rollback(agent, capability, checkpoint, resource)
        .expect("rollback event should fit");

    let events = kernel.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, EventKind::CheckpointCreated);
    assert_eq!(events[1].kind, EventKind::RollbackRequested);
}

#[test]
fn action_and_verify_syscalls_record_action_lifecycle() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(88);
    let action = ActionId::new(3);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");

    kernel
        .sys_act(agent, capability, action, resource)
        .expect("act should be authorized");
    kernel
        .sys_verify(agent, capability, action, resource)
        .expect("verify should be authorized");

    let events = kernel.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, EventKind::ActionExecuted);
    assert_eq!(events[1].kind, EventKind::VerificationRequested);
    assert_eq!(events[0].action, Some(action));
    assert_eq!(events[1].action, Some(action));
}

#[test]
fn delegate_syscall_records_task_delegation() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(99);
    let target_agent = AgentId::new(100);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            agent,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("capability should fit");
    let task = kernel
        .sys_create_task(agent, capability, resource)
        .expect("task should be created");

    let event = kernel
        .sys_delegate_task(agent, capability, task, target_agent)
        .expect("delegate should be authorized");

    assert_eq!(event.kind, EventKind::DelegationRequested);
    assert_eq!(event.task, Some(task));
    assert_eq!(event.target_agent, Some(target_agent));
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Delegated);
}

#[test]
fn task_syscalls_record_full_task_lifecycle() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(101);
    let assignee = AgentId::new(102);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("owner capability should fit");
    let assignee_capability = kernel
        .sys_grant(assignee, resource, OperationSet::only(Operation::Act))
        .expect("assignee capability should fit");

    let task = kernel
        .sys_create_task(owner, owner_capability, resource)
        .expect("task should be created");
    assert_eq!(task, TaskId::new(1));
    kernel
        .sys_delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    kernel
        .sys_accept_task(assignee, task)
        .expect("task should be accepted");
    kernel
        .sys_complete_task(assignee, assignee_capability, task)
        .expect("task should be completed");
    kernel
        .sys_verify_task(owner, owner_capability, task)
        .expect("task should be verified");

    assert_eq!(kernel.tasks()[0].status, TaskStatus::Verified);
    assert_eq!(kernel.events()[0].kind, EventKind::TaskCreated);
    assert_eq!(kernel.events()[1].kind, EventKind::DelegationRequested);
    assert_eq!(kernel.events()[2].kind, EventKind::TaskAccepted);
    assert_eq!(kernel.events()[3].kind, EventKind::TaskCompleted);
    assert_eq!(kernel.events()[4].kind, EventKind::TaskVerified);
}

#[test]
fn cancel_task_syscall_records_cancelled_task() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(103);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let task = kernel
        .sys_create_task(owner, capability, resource)
        .expect("task should be created");

    let event = kernel
        .sys_cancel_task(owner, capability, task)
        .expect("task should be cancelled");

    assert_eq!(event.kind, EventKind::TaskCancelled);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Cancelled);
}

#[test]
fn scheduler_syscalls_enqueue_dispatch_and_yield_tasks() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(200);
    let first_agent = AgentId::new(201);
    let second_agent = AgentId::new(202);
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("owner capability should fit");
    let first = kernel
        .sys_create_task(owner, owner_capability, resource)
        .expect("first task should be created");
    let second = kernel
        .sys_create_task(owner, owner_capability, resource)
        .expect("second task should be created");
    kernel
        .sys_delegate_task(owner, owner_capability, first, first_agent)
        .expect("first task should delegate");
    kernel
        .sys_delegate_task(owner, owner_capability, second, second_agent)
        .expect("second task should delegate");
    kernel
        .sys_accept_task(first_agent, first)
        .expect("first task should accept");
    kernel
        .sys_accept_task(second_agent, second)
        .expect("second task should accept");

    kernel
        .sys_enqueue_task(first_agent, first)
        .expect("first task should enqueue");
    kernel
        .sys_yield_task(second_agent, second)
        .expect("second task should yield into queue");
    let dispatched = kernel
        .sys_dispatch_next(first_agent)
        .expect("first task should dispatch");

    assert_eq!(dispatched, first);
    assert_eq!(
        kernel.run_queue(),
        &[RunQueueEntry {
            task: second,
            agent: second_agent,
        }]
    );
    assert_eq!(
        kernel
            .events()
            .last()
            .expect("dispatch event should exist")
            .kind,
        EventKind::TaskDispatched
    );
}
