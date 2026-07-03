use agent_kernel::AgentKernel;
use agent_kernel_core::{
    ActionId, ActionStatus, AgentId, CapabilityId, CheckpointId, CheckpointStatus, EventKind,
    IntentId, IntentKind, KernelError, Operation, OperationSet, ResourceId, ResourceKind,
    RunQueueEntry, TaskId, TaskStatus, VerificationRequirement,
};

type TestKernel = AgentKernel<2, 4, 6, 64, 8, 8, 8, 8, 8, 4>;

fn declare_action_intent(
    kernel: &mut TestKernel,
    agent: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
) -> IntentId {
    kernel
        .sys_declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared")
}

#[test]
fn kernel_starts_with_empty_event_log() {
    let kernel = TestKernel::new();

    assert!(kernel.events().is_empty());
    assert!(kernel.checkpoints().is_empty());
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
    assert_eq!(kernel.events().len(), 2);
    assert_eq!(kernel.events()[0].kind, EventKind::CapabilityGranted);
    assert_eq!(kernel.events()[1].kind, EventKind::Observation);
    assert_eq!(kernel.observations().len(), 1);
    assert_eq!(
        kernel.events()[1].observation,
        Some(kernel.observations()[0].id)
    );
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
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, EventKind::CapabilityGranted);
    assert_eq!(events[1].kind, EventKind::CheckpointCreated);
    assert_eq!(events[2].kind, EventKind::RollbackRequested);
    assert_eq!(kernel.checkpoints().len(), 1);
    assert_eq!(kernel.checkpoints()[0].id, checkpoint);
    assert_eq!(
        kernel.checkpoints()[0].status,
        CheckpointStatus::RollbackRequested
    );
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
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, EventKind::CapabilityGranted);
    assert_eq!(events[1].kind, EventKind::ActionExecuted);
    assert_eq!(events[2].kind, EventKind::VerificationRequested);
    assert_eq!(events[1].action, Some(action));
    assert_eq!(events[2].action, Some(action));
    assert_eq!(kernel.actions().len(), 1);
    assert_eq!(
        kernel.actions()[0].status,
        ActionStatus::VerificationRequested
    );
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
    let intent = declare_action_intent(&mut kernel, agent, capability, resource);
    let task = kernel
        .sys_create_task(agent, capability, intent)
        .expect("task should be created");

    let event = kernel
        .sys_delegate_task(agent, capability, task, target_agent)
        .expect("delegate should be authorized");

    assert_eq!(event.kind, EventKind::DelegationRequested);
    assert_eq!(event.task, Some(task));
    assert_eq!(event.target_agent, Some(target_agent));
    assert_eq!(kernel.tasks()[0].delegated_capability, event.capability);
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
    let intent = declare_action_intent(&mut kernel, owner, owner_capability, resource);
    let task = kernel
        .sys_create_task(owner, owner_capability, intent)
        .expect("task should be created");
    assert_eq!(task, TaskId::new(1));
    kernel
        .sys_delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    let assignee_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive assignee capability");
    kernel
        .sys_accept_task(assignee, task)
        .expect("task should be accepted");
    kernel
        .sys_enqueue_task(assignee, task)
        .expect("task should enqueue");
    kernel
        .sys_dispatch_next(assignee)
        .expect("task should dispatch");
    kernel
        .sys_complete_task(assignee, assignee_capability, task)
        .expect("task should be completed");
    kernel
        .sys_verify_task(owner, owner_capability, task)
        .expect("task should be verified");

    assert_eq!(kernel.tasks()[0].status, TaskStatus::Verified);
    assert_eq!(kernel.events()[0].kind, EventKind::CapabilityGranted);
    assert_eq!(kernel.events()[1].kind, EventKind::IntentDeclared);
    assert_eq!(kernel.events()[2].kind, EventKind::TaskCreated);
    assert_eq!(kernel.events()[3].kind, EventKind::IntentBound);
    assert_eq!(kernel.events()[4].kind, EventKind::CapabilityDerived);
    assert_eq!(kernel.events()[5].kind, EventKind::DelegationRequested);
    assert_eq!(kernel.events()[6].kind, EventKind::TaskAccepted);
    assert_eq!(kernel.events()[7].kind, EventKind::TaskQueued);
    assert_eq!(kernel.events()[8].kind, EventKind::TaskDispatched);
    assert_eq!(kernel.events()[9].kind, EventKind::TaskCompleted);
    assert_eq!(kernel.events()[10].kind, EventKind::TaskVerified);
    assert_eq!(kernel.events()[11].kind, EventKind::IntentFulfilled);
    for event in &kernel.events()[2..=11] {
        assert_eq!(event.intent, Some(intent));
    }
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
    let intent = declare_action_intent(&mut kernel, owner, capability, resource);
    let task = kernel
        .sys_create_task(owner, capability, intent)
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
    let first_intent = declare_action_intent(&mut kernel, owner, owner_capability, resource);
    let first = kernel
        .sys_create_task(owner, owner_capability, first_intent)
        .expect("first task should be created");
    let second_intent = declare_action_intent(&mut kernel, owner, owner_capability, resource);
    let second = kernel
        .sys_create_task(owner, owner_capability, second_intent)
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
        .sys_enqueue_task(second_agent, second)
        .expect("second task should enqueue");
    let dispatched = kernel
        .sys_dispatch_next(first_agent)
        .expect("first task should dispatch");
    kernel
        .sys_yield_task(first_agent, first)
        .expect("running task should yield into queue");

    assert_eq!(dispatched, first);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(
        kernel.run_queue(),
        &[
            RunQueueEntry {
                task: second,
                agent: second_agent,
            },
            RunQueueEntry {
                task: first,
                agent: first_agent,
            }
        ]
    );
    assert_eq!(
        kernel
            .events()
            .last()
            .expect("yield event should exist")
            .kind,
        EventKind::TaskYielded
    );
}

#[test]
fn completing_task_before_dispatch_is_rejected_by_facade() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(203);
    let assignee = AgentId::new(204);
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
    let intent = declare_action_intent(&mut kernel, owner, owner_capability, resource);
    let task = kernel
        .sys_create_task(owner, owner_capability, intent)
        .expect("task should be created");
    kernel
        .sys_delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive assignee capability");
    kernel
        .sys_accept_task(assignee, task)
        .expect("task should accept");
    let events_before = kernel.events().len();

    let result = kernel.sys_complete_task(assignee, assignee_capability, task);

    assert_eq!(result, Err(KernelError::TaskStatusMismatch));
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(kernel.events().len(), events_before);
}
