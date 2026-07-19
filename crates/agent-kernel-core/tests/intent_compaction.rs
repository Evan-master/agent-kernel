use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, EventKind, IntentId,
    IntentKind, KernelCore, KernelError, MessageKind, MessagePayload, Operation, OperationSet,
    ResourceId, ResourceKind, TaskId, VerificationRequirement,
};

type TestCore<const EVENTS: usize> =
    KernelCore<2, 1, 5, EVENTS, 0, 32, 0, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 2>;

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

fn declare_intent<const EVENTS: usize>(core: &mut TestCore<EVENTS>, fixture: Fixture) -> IntentId {
    core.declare_intent(
        fixture.supervisor,
        fixture.authority,
        fixture.resource,
        IntentKind::Act,
        VerificationRequirement::Required,
    )
    .expect("intent declares")
}

fn cancel_intent_task<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
) -> (IntentId, TaskId) {
    let intent = declare_intent(core, fixture);
    let task = core
        .create_task(fixture.supervisor, fixture.authority, intent)
        .expect("task creates");
    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("task cancels");
    (intent, task)
}

fn retire_cancelled_task<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
) -> IntentId {
    let (intent, task) = cancel_intent_task(core, fixture);
    core.compact_task_prefix(fixture.supervisor, fixture.authority, task)
        .expect("cancelled task compacts");
    intent
}

#[test]
fn terminal_prefix_compaction_reuses_capacity_and_preserves_monotonic_ids() {
    let (mut core, fixture) = prepared::<64>();
    let first = retire_cancelled_task(&mut core, fixture);
    let first_record = core.intent(first).expect("first intent exists");
    let second = declare_intent(&mut core, fixture);
    let event_start = core.events().len();

    let receipt = core
        .compact_intent_prefix(fixture.supervisor, fixture.authority, first)
        .expect("terminal prefix compacts");

    assert_eq!(receipt.first(), first);
    assert_eq!(receipt.through(), first);
    assert_eq!(receipt.count(), 1);
    assert_eq!(core.intent_capacity(), 2);
    assert_eq!(core.intent(first), Err(KernelError::IntentNotFound));
    assert_eq!(core.intents(), [core.intent(second).unwrap()]);
    let event = core.events().get(event_start).expect("event exists");
    assert_eq!(event.kind, EventKind::IntentCompacted);
    assert_eq!(event.agent, fixture.supervisor);
    assert_eq!(event.capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.resource, Some(first_record.resource));
    assert_eq!(event.intent, Some(first));
    assert_eq!(event.intent_kind, Some(first_record.kind));
    assert_eq!(event.verification, first_record.verification);
    assert_eq!(event.target_agent, Some(first_record.owner));

    let third = declare_intent(&mut core, fixture);
    assert_eq!(third, IntentId::new(3));
    assert_eq!(
        core.intents()
            .iter()
            .map(|intent| intent.id)
            .collect::<Vec<_>>(),
        [second, third]
    );
}

#[test]
fn compaction_rejects_nonterminal_unauthorized_and_full_event_log_atomically() {
    let (mut core, fixture) = prepared::<24>();
    let intent = declare_intent(&mut core, fixture);
    let initial = core.intent(intent).expect("intent exists");
    let event_count = core.events().len();

    assert_eq!(
        core.compact_intent_prefix(fixture.supervisor, fixture.authority, IntentId::new(0)),
        Err(KernelError::IntentNotFound)
    );
    assert_eq!(
        core.compact_intent_prefix(fixture.supervisor, fixture.authority, intent),
        Err(KernelError::IntentCompactionNotReady)
    );
    assert_eq!(core.intents(), [initial]);
    assert_eq!(core.events().len(), event_count);

    let task = core
        .create_task(fixture.supervisor, fixture.authority, intent)
        .expect("task creates");
    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("task cancels");
    core.compact_task_prefix(fixture.supervisor, fixture.authority, task)
        .expect("task compacts");
    let observe_only = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.supervisor,
            OperationSet::only(Operation::Observe),
        )
        .expect("attenuated capability derives");
    let terminal = core.intent(intent).expect("terminal intent exists");
    let event_count = core.events().len();
    assert_eq!(
        core.compact_intent_prefix(fixture.supervisor, observe_only, intent),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.intents(), [terminal]);
    assert_eq!(core.events().len(), event_count);

    while core.events().len() < 24 {
        core.observe(fixture.supervisor, fixture.authority, fixture.resource)
            .expect("filler observation fits");
    }
    assert_eq!(
        core.compact_intent_prefix(fixture.supervisor, fixture.authority, intent),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.intents(), [terminal]);
    assert_eq!(core.events().len(), 24);
}

#[test]
fn active_task_must_leave_before_its_terminal_intent() {
    let (mut core, fixture) = prepared::<64>();
    let (intent, task) = cancel_intent_task(&mut core, fixture);

    assert_eq!(
        core.compact_intent_prefix(fixture.supervisor, fixture.authority, intent),
        Err(KernelError::IntentCompactionReferenced)
    );
    core.compact_task_prefix(fixture.supervisor, fixture.authority, task)
        .expect("task compacts first");
    core.compact_intent_prefix(fixture.supervisor, fixture.authority, intent)
        .expect("intent compacts after task");
    assert!(core.intents().is_empty());
}

#[test]
fn unacknowledged_message_reference_blocks_intent_compaction() {
    let (mut core, fixture) = prepared::<64>();
    let intent = retire_cancelled_task(&mut core, fixture);
    let message = core
        .send_message(
            fixture.supervisor,
            fixture.worker,
            MessageKind::Notify,
            MessagePayload {
                intent: Some(intent),
                ..MessagePayload::empty()
            },
        )
        .expect("intent message sends");

    assert_eq!(
        core.compact_intent_prefix(fixture.supervisor, fixture.authority, intent),
        Err(KernelError::IntentCompactionReferenced)
    );
    assert_eq!(core.receive_message(fixture.worker), Ok(message));
    assert_eq!(
        core.compact_intent_prefix(fixture.supervisor, fixture.authority, intent),
        Err(KernelError::IntentCompactionReferenced)
    );
    core.acknowledge_message(fixture.worker, message)
        .expect("message acknowledges");
    core.compact_intent_prefix(fixture.supervisor, fixture.authority, intent)
        .expect("historical message permits compaction");
}

#[test]
fn historical_agent_entry_reference_permits_intent_compaction() {
    let (mut core, fixture) = prepared::<64>();
    let intent = declare_intent(&mut core, fixture);
    let task = core
        .create_task(fixture.supervisor, fixture.authority, intent)
        .expect("task creates");
    core.delegate_task(fixture.supervisor, fixture.authority, task, fixture.worker)
        .expect("task delegates");
    let task_capability = core
        .task(task)
        .expect("task exists")
        .delegated_capability
        .expect("task capability exists");
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
    core.cancel_task(fixture.supervisor, fixture.authority, task)
        .expect("task cancels");
    core.compact_task_prefix(fixture.supervisor, fixture.authority, task)
        .expect("task compacts");

    assert_eq!(
        core.agent_entry(fixture.worker).unwrap().intent,
        Some(intent)
    );
    core.compact_intent_prefix(fixture.supervisor, fixture.authority, intent)
        .expect("historical launch entry permits compaction");
    assert_eq!(core.intent(intent), Err(KernelError::IntentNotFound));
}
