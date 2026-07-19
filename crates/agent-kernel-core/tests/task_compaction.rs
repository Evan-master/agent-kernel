use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, EventKind, IntentKind,
    KernelCore, KernelError, MessageKind, MessagePayload, NamespaceKey, NamespaceObject, Operation,
    OperationSet, ResourceId, ResourceKind, RuntimeAdmissionFailure, TaskId, TaskStatus,
    VerificationRequirement,
};

type TestCore<const EVENTS: usize> =
    KernelCore<2, 1, 5, EVENTS, 0, 8, 0, 4, 2, 2, 2, 0, 1, 0, 0, 0, 2, 2, 0, 0, 0, 0, 2>;

#[derive(Copy, Clone)]
struct Fixture {
    supervisor: AgentId,
    worker: AgentId,
    authority: CapabilityId,
    resource: ResourceId,
}

fn prepared<const EVENTS: usize>() -> (TestCore<EVENTS>, Fixture) {
    let mut core = TestCore::new();
    let supervisor = AgentId::new(1);
    let worker = AgentId::new(2);
    core.register_agent(supervisor)
        .expect("supervisor registers");
    core.register_agent(worker).expect("worker registers");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource registers");
    let authority = core
        .grant_capability(
            supervisor,
            resource,
            OperationSet::only(Operation::Observe)
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify)
                .with(Operation::Rollback),
        )
        .expect("authority grants");
    let image = core
        .register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("supervisor image registers");
    core.verify_agent_image(supervisor, authority, image)
        .expect("supervisor image verifies");
    core.launch_agent(
        supervisor,
        authority,
        resource,
        image,
        AgentEntryKind::Supervisor,
        None,
    )
    .expect("supervisor launches");
    (
        core,
        Fixture {
            supervisor,
            worker,
            authority,
            resource,
        },
    )
}

fn create_task<const EVENTS: usize>(core: &mut TestCore<EVENTS>, fixture: Fixture) -> TaskId {
    let intent = core
        .declare_intent(
            fixture.supervisor,
            fixture.authority,
            fixture.resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent declares");
    core.create_task(fixture.supervisor, fixture.authority, intent)
        .expect("task creates")
}

fn cancel_task<const EVENTS: usize>(core: &mut TestCore<EVENTS>, fixture: Fixture) -> TaskId {
    let task = create_task(core, fixture);
    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("task cancels");
    task
}

fn accepted_worker_task<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
) -> (TaskId, CapabilityId) {
    let task = create_task(core, fixture);
    core.delegate_task(fixture.supervisor, fixture.authority, task, fixture.worker)
        .expect("task delegates");
    let task_capability = core
        .task(task)
        .expect("task remains active")
        .delegated_capability
        .expect("task has delegated capability");
    let image = core
        .register_agent_image(
            fixture.supervisor,
            fixture.authority,
            fixture.resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([2; 32]),
            1,
            1,
        )
        .expect("worker image registers");
    core.verify_agent_image(fixture.supervisor, fixture.authority, image)
        .expect("worker image verifies");
    core.launch_task_agent(
        fixture.worker,
        task_capability,
        task,
        image,
        AgentEntryKind::Worker,
    )
    .expect("worker launches");
    core.accept_task(fixture.worker, task)
        .expect("worker accepts task");
    (task, task_capability)
}

#[test]
fn terminal_prefix_compaction_reuses_capacity_and_invalidates_dispatch_permits() {
    let (mut core, fixture) = prepared::<64>();
    let first = cancel_task(&mut core, fixture);
    let first_record = core.task(first).expect("first task exists");
    let (second, _) = accepted_worker_task(&mut core, fixture);
    core.enqueue_task(fixture.worker, second)
        .expect("second task queues");
    let permit = core
        .prepare_next_ready_dispatch_with_quantum(3)
        .expect("dispatch permit prepares");
    assert_eq!(core.task_capacity(), 2);
    assert_eq!(core.tasks().len(), 2);
    let event_start = core.events().len();

    let receipt = core
        .compact_task_prefix(fixture.supervisor, fixture.authority, first)
        .expect("terminal prefix compacts");

    assert_eq!(receipt.first(), first);
    assert_eq!(receipt.through(), first);
    assert_eq!(receipt.count(), 1);
    assert_eq!(core.task(first), Err(KernelError::TaskNotFound));
    assert_eq!(core.tasks().len(), 1);
    assert_eq!(core.tasks()[0].id, second);
    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(
        core.commit_ready_dispatch(permit),
        Err(KernelError::TaskDispatchPermitStale)
    );
    assert_eq!(core.run_queue()[0].task, second);

    let event = core
        .events()
        .get(event_start)
        .expect("compaction event exists");
    assert_eq!(event.kind, EventKind::TaskCompacted);
    assert_eq!(event.agent, fixture.supervisor);
    assert_eq!(event.capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.resource, Some(first_record.resource));
    assert_eq!(event.intent, Some(first_record.intent));
    assert_eq!(event.task, Some(first));
    assert_eq!(event.target_agent, first_record.assignee);
    assert_eq!(event.task_result, first_record.result);
    assert_eq!(event.task_ticks, Some(first_record.run_ticks));
    assert_eq!(event.fault, first_record.last_fault);

    let third = create_task(&mut core, fixture);
    assert_eq!(third, TaskId::new(3));
    assert_eq!(
        core.tasks().iter().map(|task| task.id).collect::<Vec<_>>(),
        [second, third]
    );
}

#[test]
fn compaction_rejects_nonterminal_unauthorized_and_full_event_log_atomically() {
    let (mut core, fixture) = prepared::<16>();
    let task = create_task(&mut core, fixture);
    let record = core.task(task).expect("task exists");
    let event_count = core.events().len();

    assert_eq!(
        core.compact_task_prefix(fixture.supervisor, fixture.authority, TaskId::new(0)),
        Err(KernelError::TaskNotFound)
    );
    assert_eq!(
        core.compact_task_prefix(fixture.supervisor, fixture.authority, task),
        Err(KernelError::TaskCompactionNotReady)
    );
    assert_eq!(core.tasks(), [record]);
    assert_eq!(core.events().len(), event_count);

    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("task cancels");
    let observe_only = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.supervisor,
            OperationSet::only(Operation::Observe),
        )
        .expect("attenuated capability derives");
    let terminal = core.task(task).expect("terminal task exists");
    let event_count = core.events().len();
    assert_eq!(
        core.compact_task_prefix(fixture.supervisor, observe_only, task),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.tasks(), [terminal]);
    assert_eq!(core.events().len(), event_count);

    while core.events().len() < 16 {
        core.observe(fixture.supervisor, fixture.authority, fixture.resource)
            .expect("filler observation fits");
    }
    assert_eq!(
        core.compact_task_prefix(fixture.supervisor, fixture.authority, task),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.tasks(), [terminal]);
    assert_eq!(core.events().len(), 16);
}

#[test]
fn live_namespace_and_message_references_must_be_retired_first() {
    let (mut core, fixture) = prepared::<64>();
    let task = create_task(&mut core, fixture);
    let entry = core
        .bind_namespace_entry(
            fixture.supervisor,
            fixture.authority,
            fixture.resource,
            NamespaceKey::new(7),
            NamespaceObject::Task(task),
        )
        .expect("task binds into namespace");
    let message = core
        .send_message(
            fixture.supervisor,
            fixture.worker,
            MessageKind::Notify,
            MessagePayload {
                task: Some(task),
                ..MessagePayload::empty()
            },
        )
        .expect("task message sends");
    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("task cancels");
    let event_count = core.events().len();

    assert_eq!(
        core.compact_task_prefix(fixture.supervisor, fixture.authority, task),
        Err(KernelError::TaskCompactionReferenced)
    );
    assert_eq!(core.events().len(), event_count);

    core.rebind_namespace_entry(
        fixture.supervisor,
        fixture.authority,
        entry,
        NamespaceObject::Resource(fixture.resource),
    )
    .expect("namespace reference rebinds");
    assert_eq!(core.receive_message(fixture.worker), Ok(message));
    core.acknowledge_message(fixture.worker, message)
        .expect("message acknowledges");
    core.compact_task_prefix(fixture.supervisor, fixture.authority, task)
        .expect("historical references permit compaction");
    assert!(core.tasks().is_empty());
}

#[test]
fn runtime_admission_compaction_must_precede_task_compaction() {
    let (mut core, fixture) = prepared::<64>();
    let (task, _) = accepted_worker_task(&mut core, fixture);
    let admission = core
        .request_runtime_admission(fixture.supervisor, fixture.authority, fixture.worker, task)
        .expect("runtime admission requests");
    let permit = core
        .prepare_next_runtime_admission()
        .expect("runtime admission prepares");
    core.reject_runtime_admission(permit, RuntimeAdmissionFailure::AllocationUnavailable)
        .expect("runtime admission rejects");
    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("task cancels");

    assert_eq!(
        core.compact_task_prefix(fixture.supervisor, fixture.authority, task),
        Err(KernelError::TaskCompactionReferenced)
    );
    core.compact_runtime_admission_prefix(fixture.supervisor, fixture.authority, admission)
        .expect("runtime admission compacts first");
    core.compact_task_prefix(fixture.supervisor, fixture.authority, task)
        .expect("task compacts after reference removal");
    assert!(core.tasks().is_empty());
}

#[test]
fn cancelling_a_queued_task_removes_its_run_queue_entry() {
    let (mut core, fixture) = prepared::<64>();
    let (task, _) = accepted_worker_task(&mut core, fixture);
    core.enqueue_task(fixture.worker, task)
        .expect("task queues");

    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("queued task cancels");

    assert!(core.run_queue().is_empty());
    assert_eq!(
        core.task(task).expect("task remains active").status,
        TaskStatus::Cancelled
    );
}
