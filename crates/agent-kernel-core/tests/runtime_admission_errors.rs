use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind, CapabilityId,
    IntentKind, KernelCore, KernelError, Operation, OperationSet, ResourceId, ResourceKind,
    SignalKey, TaskId, TaskStatus, VerificationRequirement,
};

type TestCore = KernelCore<3, 4, 16, 96, 0, 0, 0, 6, 6, 6, 0, 0, 0, 1, 0, 0, 1>;

struct PreparedTask {
    owner: AgentId,
    worker: AgentId,
    resource: ResourceId,
    owner_capability: CapabilityId,
    worker_capability: CapabilityId,
    task: TaskId,
}

fn prepare_worker_task(core: &mut TestCore) -> PreparedTask {
    let owner = AgentId::new(1);
    let worker = AgentId::new(2);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(worker).expect("worker should register");
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
    let task = create_delegated_task(core, owner, worker, resource, owner_capability);
    let worker_capability = core.tasks()[0]
        .delegated_capability
        .expect("delegation should derive worker capability");

    PreparedTask {
        owner,
        worker,
        resource,
        owner_capability,
        worker_capability,
        task,
    }
}

fn create_delegated_task(
    core: &mut TestCore,
    owner: AgentId,
    worker: AgentId,
    resource: ResourceId,
    owner_capability: CapabilityId,
) -> TaskId {
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
    core.delegate_task(owner, owner_capability, task, worker)
        .expect("task should delegate");
    task
}

fn register_worker_image(core: &mut TestCore, prepared: &PreparedTask) -> AgentImageId {
    let image = core
        .register_agent_image(
            prepared.owner,
            prepared.owner_capability,
            prepared.resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    core.verify_agent_image(prepared.owner, prepared.owner_capability, image)
        .expect("worker image should verify");
    image
}

#[test]
fn enqueue_rejects_unlaunched_assignee_without_queue_or_event() {
    let mut core = TestCore::new();
    let prepared = prepare_worker_task(&mut core);
    core.accept_task(prepared.worker, prepared.task)
        .expect("worker should accept task");
    let events_after_accept = core.events().len();

    let result = core.enqueue_task(prepared.worker, prepared.task);

    assert_eq!(result, Err(KernelError::AgentNotLaunched));
    assert!(core.run_queue().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.events().len(), events_after_accept);
}

#[test]
fn task_scoped_entry_rejects_another_task_without_queue_or_event() {
    let mut core = TestCore::new();
    let prepared = prepare_worker_task(&mut core);
    let second_task = create_delegated_task(
        &mut core,
        prepared.owner,
        prepared.worker,
        prepared.resource,
        prepared.owner_capability,
    );
    let image = register_worker_image(&mut core, &prepared);
    core.launch_task_agent(
        prepared.worker,
        prepared.worker_capability,
        prepared.task,
        image,
        AgentEntryKind::Worker,
    )
    .expect("worker should launch for first task");
    core.accept_task(prepared.worker, second_task)
        .expect("worker should accept second task");
    let events_after_accept = core.events().len();

    let result = core.enqueue_task(prepared.worker, second_task);

    assert_eq!(result, Err(KernelError::AgentEntryScopeMismatch));
    assert!(core.run_queue().is_empty());
    assert_eq!(core.events().len(), events_after_accept);
}

#[test]
fn revoked_launch_capability_blocks_running_task_ticks() {
    let mut core = TestCore::new();
    let prepared = prepare_worker_task(&mut core);
    let image = register_worker_image(&mut core, &prepared);
    core.launch_task_agent(
        prepared.worker,
        prepared.worker_capability,
        prepared.task,
        image,
        AgentEntryKind::Worker,
    )
    .expect("worker should launch");
    core.accept_task(prepared.worker, prepared.task)
        .expect("worker should accept");
    core.enqueue_task(prepared.worker, prepared.task)
        .expect("worker should enqueue");
    core.dispatch_next(prepared.worker)
        .expect("worker should dispatch");
    core.revoke_capability(prepared.worker_capability)
        .expect("revocation should record");
    let events_after_revoke = core.events().len();

    let result = core.tick_task(prepared.worker, prepared.task);

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.tasks()[0].run_ticks, 0);
    assert_eq!(core.events().len(), events_after_revoke);
}

#[test]
fn signal_wakeup_rejects_revoked_waiter_launch_authority_without_requeue() {
    let mut core = TestCore::new();
    let prepared = prepare_worker_task(&mut core);
    let signal = SignalKey::new(1);
    let image = register_worker_image(&mut core, &prepared);
    core.launch_task_agent(
        prepared.worker,
        prepared.worker_capability,
        prepared.task,
        image,
        AgentEntryKind::Worker,
    )
    .expect("worker should launch");
    core.accept_task(prepared.worker, prepared.task)
        .expect("worker should accept");
    core.enqueue_task(prepared.worker, prepared.task)
        .expect("worker should enqueue");
    core.dispatch_next_with_quantum(prepared.worker, 2)
        .expect("worker should dispatch");
    core.wait_task(
        prepared.worker,
        prepared.worker_capability,
        prepared.task,
        prepared.resource,
        signal,
    )
    .expect("worker should wait");
    core.revoke_capability(prepared.worker_capability)
        .expect("revocation should record");
    let events_after_revoke = core.events().len();

    let result = core.emit_signal(
        prepared.owner,
        prepared.owner_capability,
        prepared.resource,
        signal,
    );

    assert_eq!(result, Err(KernelError::CapabilityRevoked));
    assert!(core.run_queue().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert!(core.waiters()[0].active);
    assert_eq!(core.events().len(), events_after_revoke);
}
