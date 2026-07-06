use agent_kernel_core::{
    AgentEntryKind, AgentId, EventKind, IntentKind, KernelCore, Operation, OperationSet,
    ResourceId, ResourceKind, TaskId, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<3, 4, 12, 80, 0, 0, 0, 4, 4, 4>;

struct PreparedTask {
    owner: AgentId,
    assignee: AgentId,
    resource: ResourceId,
    owner_capability: agent_kernel_core::CapabilityId,
    assignee_capability: agent_kernel_core::CapabilityId,
    task: TaskId,
}

fn delegated_task(core: &mut TestCore, assignee: AgentId) -> PreparedTask {
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    if assignee != owner {
        core.register_agent(assignee)
            .expect("assignee should register");
    }
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
                .with(Operation::Rollback),
        )
        .expect("owner capability should fit");
    let intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should create");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");

    PreparedTask {
        owner,
        assignee,
        resource,
        owner_capability,
        assignee_capability,
        task,
    }
}

#[test]
fn resource_scoped_launch_admits_same_resource_task_runtime() {
    let mut core = TestCore::new();
    let prepared = delegated_task(&mut core, AgentId::new(1));
    core.launch_agent(
        prepared.owner,
        prepared.owner_capability,
        prepared.resource,
        AgentEntryKind::Supervisor,
        None,
    )
    .expect("resource-scoped launch should succeed");
    core.accept_task(prepared.assignee, prepared.task)
        .expect("self-assigned task should accept");

    core.enqueue_task(prepared.assignee, prepared.task)
        .expect("launched agent should enqueue task");
    let dispatched = core
        .dispatch_next(prepared.assignee)
        .expect("launched agent should dispatch task");

    assert_eq!(dispatched, prepared.task);
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
}

#[test]
fn task_scoped_launch_admits_delegated_worker_without_root_authority() {
    let mut core = TestCore::new();
    let worker = AgentId::new(2);
    let prepared = delegated_task(&mut core, worker);

    let event = core
        .launch_task_agent(
            worker,
            prepared.assignee_capability,
            prepared.task,
            AgentEntryKind::Worker,
        )
        .expect("task-scoped launch should succeed");
    core.accept_task(worker, prepared.task)
        .expect("worker should accept delegated task");
    core.enqueue_task(worker, prepared.task)
        .expect("launched worker should enqueue task");
    let dispatched = core
        .dispatch_next(worker)
        .expect("launched worker should dispatch task");
    let entry = core.agent_entry(worker).expect("worker entry should exist");

    assert_eq!(event.kind, EventKind::AgentLaunched);
    assert_eq!(event.task, Some(prepared.task));
    assert_eq!(event.intent, Some(core.tasks()[0].intent));
    assert_eq!(entry.task, Some(prepared.task));
    assert_eq!(entry.intent, Some(core.tasks()[0].intent));
    assert_eq!(dispatched, prepared.task);
}
