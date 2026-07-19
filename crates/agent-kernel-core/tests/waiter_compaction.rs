use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, EventKind, IntentKind,
    KernelCore, KernelError, MessageKind, MessagePayload, MessageReceiveOutcome, Operation,
    OperationSet, ResourceId, ResourceKind, SignalKey, TaskId, VerificationRequirement, WaiterId,
    WaiterKind,
};

type TestCore<const EVENTS: usize, const WAITERS: usize> =
    KernelCore<1, 2, 6, EVENTS, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, WAITERS, 1>;

#[derive(Copy, Clone)]
struct Fixture {
    actor: AgentId,
    root: ResourceId,
    root_authority: CapabilityId,
    resource: ResourceId,
    authority: CapabilityId,
    task_authority: CapabilityId,
    task: TaskId,
}

fn all_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

fn running_fixture<const EVENTS: usize, const WAITERS: usize>(
    kind: AgentEntryKind,
    child_resource: bool,
) -> (TestCore<EVENTS, WAITERS>, Fixture) {
    let mut core = TestCore::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).expect("actor registers");
    let root = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("root resource registers");
    let root_authority = core
        .grant_capability(actor, root, all_operations())
        .expect("root authority grants");
    let (resource, authority) = if child_resource {
        let child = core
            .create_resource(
                actor,
                ResourceKind::Service,
                Some((root, root_authority)),
                all_operations(),
            )
            .expect("child resource creates");
        (child.resource, child.capability)
    } else {
        (root, root_authority)
    };
    let intent = core
        .declare_intent(
            actor,
            authority,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent declares");
    let task = core
        .create_task(actor, authority, intent)
        .expect("task creates");
    core.delegate_task(actor, authority, task, actor)
        .expect("task delegates to actor");
    let task_authority = core
        .task(task)
        .expect("task remains resident")
        .delegated_capability
        .expect("task authority exists");
    let image_kind = if kind == AgentEntryKind::Supervisor {
        AgentImageKind::Supervisor
    } else {
        AgentImageKind::Worker
    };
    let image = core
        .register_agent_image(
            actor,
            authority,
            resource,
            image_kind,
            AgentImageDigest::new([0x38; 32]),
            1,
            1,
        )
        .expect("image registers");
    core.verify_agent_image(actor, authority, image)
        .expect("image verifies");
    core.launch_task_agent(actor, task_authority, task, image, kind)
        .expect("actor launches");
    core.accept_task(actor, task).expect("task accepts");
    core.enqueue_task(actor, task).expect("task queues");
    core.dispatch_next_with_quantum(actor, 2)
        .expect("task dispatches");
    (
        core,
        Fixture {
            actor,
            root,
            root_authority,
            resource,
            authority,
            task_authority,
            task,
        },
    )
}

fn wait_and_wake_signal<const EVENTS: usize, const WAITERS: usize>(
    core: &mut TestCore<EVENTS, WAITERS>,
    fixture: Fixture,
    raw_signal: u64,
) -> WaiterId {
    let signal = SignalKey::new(raw_signal);
    let waiter = core
        .wait_task(
            fixture.actor,
            fixture.task_authority,
            fixture.task,
            fixture.resource,
            signal,
        )
        .expect("running task waits");
    core.emit_signal(fixture.actor, fixture.authority, fixture.resource, signal)
        .expect("signal wakes task");
    waiter
}

#[test]
fn inactive_prefix_compacts_stably_and_reuses_capacity_with_monotonic_id() {
    let (mut core, fixture) = running_fixture::<96, 2>(AgentEntryKind::Supervisor, false);
    let first = wait_and_wake_signal(&mut core, fixture, 11);
    core.dispatch_next_with_quantum(fixture.actor, 2)
        .expect("woken task redispatches");
    let MessageReceiveOutcome::Waiting(second) = core
        .receive_or_wait_message(fixture.actor, fixture.task_authority, fixture.task)
        .expect("empty mailbox creates second waiter")
    else {
        panic!("empty mailbox must wait");
    };
    let retained = core.waiters()[1];
    let event_start = core.events().len();

    let first_receipt = core
        .compact_waiter_prefix(fixture.actor, fixture.root_authority, first)
        .expect("inactive prefix compacts");

    assert_eq!(first_receipt.first(), first);
    assert_eq!(first_receipt.through(), first);
    assert_eq!(first_receipt.count(), 1);
    assert_eq!(core.waiters(), [retained]);
    let event = core.events()[event_start];
    assert_eq!(event.kind, EventKind::WaiterCompacted);
    assert_eq!(event.agent, fixture.actor);
    assert_eq!(event.target_agent, Some(fixture.actor));
    assert_eq!(event.resource, Some(fixture.resource));
    assert_eq!(event.capability, Some(fixture.root_authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.task, Some(fixture.task));
    assert_eq!(event.waiter, Some(first));
    assert_eq!(event.signal, Some(SignalKey::new(11)));
    assert_eq!(event.waiter_kind, Some(WaiterKind::Signal));

    let message = core
        .send_message(
            fixture.actor,
            fixture.actor,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message wakes retained mailbox waiter");
    let second_event_start = core.events().len();
    let second_receipt = core
        .compact_waiter_prefix(fixture.actor, fixture.root_authority, second)
        .expect("second inactive waiter compacts");
    assert_eq!(second_receipt.first(), second);
    assert_eq!(second_receipt.count(), 1);
    assert!(core.waiters().is_empty());
    let event = core.events()[second_event_start];
    assert_eq!(event.waiter, Some(second));
    assert_eq!(event.signal, Some(SignalKey::new(0)));
    assert_eq!(event.waiter_kind, Some(WaiterKind::Mailbox));

    core.dispatch_next_with_quantum(fixture.actor, 2)
        .expect("mailbox wake redispatches task");
    assert_eq!(
        core.receive_or_wait_message(fixture.actor, fixture.task_authority, fixture.task),
        Ok(MessageReceiveOutcome::Received(message))
    );
    core.acknowledge_message(fixture.actor, message)
        .expect("message acknowledges");
    core.retire_message(fixture.actor, message)
        .expect("message retires");
    assert_eq!(
        core.receive_or_wait_message(fixture.actor, fixture.task_authority, fixture.task),
        Ok(MessageReceiveOutcome::Waiting(WaiterId::new(3)))
    );
    assert_eq!(core.waiters().len(), 1);
    assert_eq!(core.waiters()[0].id, WaiterId::new(3));
}

#[test]
fn unknown_or_active_prefix_fails_without_mutation() {
    let (mut core, fixture) = running_fixture::<64, 2>(AgentEntryKind::Supervisor, false);
    let first = wait_and_wake_signal(&mut core, fixture, 21);
    core.dispatch_next_with_quantum(fixture.actor, 2).unwrap();
    let MessageReceiveOutcome::Waiting(second) = core
        .receive_or_wait_message(fixture.actor, fixture.task_authority, fixture.task)
        .unwrap()
    else {
        panic!("second wait must block");
    };
    let waiters = core.waiters().to_vec();
    let events = core.events().len();

    assert_eq!(
        core.compact_waiter_prefix(fixture.actor, fixture.root_authority, WaiterId::new(99)),
        Err(KernelError::WaiterNotFound)
    );
    assert_eq!(
        core.compact_waiter_prefix(fixture.actor, fixture.root_authority, second),
        Err(KernelError::WaiterCompactionNotReady)
    );
    assert_eq!(core.waiters(), waiters.as_slice());
    assert_eq!(core.events().len(), events);
    assert_eq!(core.waiters()[0].id, first);
}

#[test]
fn supervisor_identity_and_rollback_authority_are_required() {
    let actor = AgentId::new(1);
    let mut unlaunched = TestCore::<8, 1>::new();
    unlaunched.register_agent(actor).unwrap();
    assert_eq!(
        unlaunched.compact_waiter_prefix(actor, CapabilityId::new(1), WaiterId::new(1)),
        Err(KernelError::AgentNotLaunched)
    );

    let (mut worker, worker_fixture) = running_fixture::<64, 1>(AgentEntryKind::Worker, false);
    let waiter = wait_and_wake_signal(&mut worker, worker_fixture, 31);
    assert_eq!(
        worker.compact_waiter_prefix(worker_fixture.actor, worker_fixture.root_authority, waiter),
        Err(KernelError::AgentEntryKindMismatch)
    );

    let (mut core, fixture) = running_fixture::<64, 1>(AgentEntryKind::Supervisor, false);
    let waiter = wait_and_wake_signal(&mut core, fixture, 32);
    let observe_only = core
        .derive_capability(
            fixture.actor,
            fixture.root_authority,
            fixture.actor,
            OperationSet::only(Operation::Observe),
        )
        .expect("attenuated authority derives");
    let waiters = core.waiters().to_vec();
    let events = core.events().len();
    assert_eq!(
        core.compact_waiter_prefix(fixture.actor, observe_only, waiter),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.waiters(), waiters.as_slice());
    assert_eq!(core.events().len(), events);
}

#[test]
fn retired_child_waiter_accepts_active_ancestor_cleanup_authority() {
    let (mut core, fixture) = running_fixture::<80, 1>(AgentEntryKind::Supervisor, true);
    let waiter = wait_and_wake_signal(&mut core, fixture, 41);

    assert_eq!(
        core.compact_waiter_prefix(fixture.actor, fixture.root_authority, waiter),
        Err(KernelError::ResourceMismatch)
    );
    core.retire_resource(fixture.actor, fixture.authority, fixture.resource)
        .expect("child resource retires");
    assert_eq!(
        core.compact_waiter_prefix(fixture.actor, fixture.root_authority, waiter)
            .expect("ancestor authority cleans retired child waiter")
            .count(),
        1
    );
    assert_eq!(fixture.root.raw(), 1);
    assert!(core.waiters().is_empty());
}

#[test]
fn full_event_log_rejects_compaction_atomically() {
    let (mut core, fixture) = running_fixture::<32, 1>(AgentEntryKind::Supervisor, false);
    let waiter = wait_and_wake_signal(&mut core, fixture, 51);
    while core.events().len() < 32 {
        core.emit_signal(
            fixture.actor,
            fixture.root_authority,
            fixture.root,
            SignalKey::new(999),
        )
        .expect("filler signal Event fits");
    }
    let waiters = core.waiters().to_vec();

    assert_eq!(
        core.compact_waiter_prefix(fixture.actor, fixture.root_authority, waiter),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.waiters(), waiters.as_slice());
    assert_eq!(core.events().len(), 32);
}
