use agent_kernel_core::{
    ActionId, AgentId, AgentStatus, CapabilityId, EventKind, FaultId, IntentId, KernelCore,
    KernelError, MessageId, MessageKind, MessagePayload, MessageStatus, NamespaceKey,
    NamespaceObject, Operation, OperationSet, ResourceId, ResourceKind, TaskId,
};

type TestCore = KernelCore<7, 2, 8, 64, 0, 0, 0, 0, 0, 0, 3, 0, 2>;

struct Fixture {
    core: TestCore,
    manager: AgentId,
    target: AgentId,
    resource: ResourceId,
    authority: CapabilityId,
}

fn setup(operations: OperationSet) -> Fixture {
    let mut core = TestCore::new();
    let manager = AgentId::new(1);
    let target = AgentId::new(9);
    core.register_agent(manager)
        .expect("manager should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("management resource should fit");
    let authority = core
        .grant_capability(manager, resource, operations)
        .expect("management authority should fit");
    core.register_managed_agent(manager, authority, resource, target)
        .expect("managed target should register");
    Fixture {
        core,
        manager,
        target,
        resource,
        authority,
    }
}

fn pending_message(fixture: &mut Fixture, payload: MessagePayload) -> MessageId {
    fixture
        .core
        .send_message(
            fixture.manager,
            fixture.target,
            MessageKind::Request,
            payload,
        )
        .expect("pending message should fit")
}

fn retire_target(fixture: &mut Fixture) {
    fixture
        .core
        .retire_managed_agent(fixture.manager, fixture.authority, fixture.target)
        .expect("quiescent managed target should retire");
    assert_eq!(fixture.core.agents()[1].status, AgentStatus::Retired);
}

#[test]
fn manager_retires_pending_message_for_retired_agent_with_complete_evidence() {
    let operations = OperationSet::empty()
        .with(Operation::Act)
        .with(Operation::Delegate);
    let mut fixture = setup(operations);
    let payload = MessagePayload {
        resource: Some(ResourceId::new(41)),
        capability: Some(CapabilityId::new(42)),
        intent: Some(IntentId::new(43)),
        task: Some(TaskId::new(44)),
        action: Some(ActionId::new(45)),
        fault: Some(FaultId::new(46)),
    };
    let message = pending_message(&mut fixture, payload);
    retire_target(&mut fixture);

    let retirement = fixture
        .core
        .retire_orphaned_message(fixture.manager, fixture.authority, message)
        .expect("manager should retire orphaned pending message");

    assert_eq!(retirement.message(), message);
    assert_eq!(retirement.actor(), fixture.manager);
    assert_eq!(retirement.authority(), fixture.authority);
    assert_eq!(retirement.management_resource(), fixture.resource);
    assert_eq!(retirement.record().sender, fixture.manager);
    assert_eq!(retirement.record().recipient, fixture.target);
    assert_eq!(retirement.record().kind, MessageKind::Request);
    assert_eq!(retirement.record().payload, payload);
    assert_eq!(retirement.record().status, MessageStatus::Pending);
    assert!(fixture.core.messages().is_empty());

    let event = fixture
        .core
        .events()
        .last()
        .expect("retirement should emit an event");
    assert_eq!(event.kind, EventKind::OrphanedMessageRetired);
    assert_eq!(event.agent, fixture.manager);
    assert_eq!(event.target_agent, Some(fixture.target));
    assert_eq!(event.message, Some(message));
    assert_eq!(event.message_kind, Some(MessageKind::Request));
    assert_eq!(event.source_capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Delegate));
    assert_eq!(event.resource, payload.resource);
    assert_eq!(event.capability, payload.capability);
    assert_eq!(event.intent, payload.intent);
    assert_eq!(event.task, payload.task);
    assert_eq!(event.action, payload.action);
    assert_eq!(event.fault, payload.fault);
}

#[test]
fn delegated_manager_can_retire_orphaned_message() {
    let mut fixture = setup(OperationSet::only(Operation::Delegate));
    let delegate = AgentId::new(2);
    fixture
        .core
        .register_agent(delegate)
        .expect("delegate should register");
    let delegated = fixture
        .core
        .derive_capability(
            fixture.manager,
            fixture.authority,
            delegate,
            OperationSet::only(Operation::Delegate),
        )
        .expect("management authority should delegate");
    let message = pending_message(&mut fixture, MessagePayload::empty());
    retire_target(&mut fixture);

    let retirement = fixture
        .core
        .retire_orphaned_message(delegate, delegated, message)
        .expect("delegated manager should retire orphaned message");

    assert_eq!(retirement.actor(), delegate);
    assert_eq!(retirement.authority(), delegated);
    assert_eq!(retirement.management_resource(), fixture.resource);
    let event = fixture.core.events().last().unwrap();
    assert_eq!(event.agent, delegate);
    assert_eq!(event.target_agent, Some(fixture.target));
    assert_eq!(event.source_capability, Some(delegated));
}

#[test]
fn active_and_suspended_recipients_are_not_orphaned() {
    let mut fixture = setup(OperationSet::only(Operation::Delegate));
    let message = pending_message(&mut fixture, MessagePayload::empty());
    let record = fixture.core.messages()[0];
    let events_before = fixture.core.events().len();

    assert_eq!(
        fixture
            .core
            .retire_orphaned_message(fixture.manager, fixture.authority, message),
        Err(KernelError::OrphanedMessageRetirementNotReady)
    );
    assert_eq!(fixture.core.messages(), &[record]);
    assert_eq!(fixture.core.events().len(), events_before);

    fixture
        .core
        .suspend_managed_agent(fixture.manager, fixture.authority, fixture.target)
        .expect("target should suspend");
    let events_before = fixture.core.events().len();
    assert_eq!(
        fixture
            .core
            .retire_orphaned_message(fixture.manager, fixture.authority, message),
        Err(KernelError::OrphanedMessageRetirementNotReady)
    );
    assert_eq!(fixture.core.messages(), &[record]);
    assert_eq!(fixture.core.events().len(), events_before);
}

#[test]
fn received_and_acknowledged_messages_are_not_orphaned_pending_mail() {
    for acknowledge in [false, true] {
        let mut fixture = setup(OperationSet::only(Operation::Delegate));
        let message = pending_message(&mut fixture, MessagePayload::empty());
        assert_eq!(fixture.core.receive_message(fixture.target), Ok(message));
        if acknowledge {
            fixture
                .core
                .acknowledge_message(fixture.target, message)
                .expect("received message should acknowledge");
        }
        retire_target(&mut fixture);
        let record = fixture.core.messages()[0];
        let events_before = fixture.core.events().len();

        assert_eq!(
            fixture
                .core
                .retire_orphaned_message(fixture.manager, fixture.authority, message),
            Err(KernelError::MessageStatusMismatch)
        );
        assert_eq!(fixture.core.messages(), &[record]);
        assert_eq!(fixture.core.events().len(), events_before);
    }
}

#[test]
fn unmanaged_retired_recipient_has_no_administrative_cleanup_authority() {
    let mut fixture = setup(OperationSet::only(Operation::Delegate));
    let trusted = AgentId::new(3);
    fixture
        .core
        .register_agent(trusted)
        .expect("trusted target should register");
    let message = fixture
        .core
        .send_message(
            fixture.manager,
            trusted,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message should fit");
    fixture
        .core
        .retire_agent(trusted)
        .expect("trusted target should retire");
    let record = *fixture.core.messages().last().unwrap();
    let events_before = fixture.core.events().len();

    assert_eq!(
        fixture
            .core
            .retire_orphaned_message(fixture.manager, fixture.authority, message),
        Err(KernelError::AgentManagementDenied)
    );
    assert_eq!(fixture.core.messages().last(), Some(&record));
    assert_eq!(fixture.core.events().len(), events_before);
}

#[test]
fn cleanup_requires_active_actor_and_exact_delegate_authority() {
    let mut fixture = setup(OperationSet::only(Operation::Delegate));
    let wrong = fixture
        .core
        .grant_capability(
            fixture.manager,
            fixture.resource,
            OperationSet::only(Operation::Act),
        )
        .expect("wrong-operation capability should fit");
    let message = pending_message(&mut fixture, MessagePayload::empty());
    retire_target(&mut fixture);
    let record = fixture.core.messages()[0];
    let events_before = fixture.core.events().len();

    assert_eq!(
        fixture
            .core
            .retire_orphaned_message(fixture.manager, wrong, message),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(fixture.core.messages(), &[record]);
    assert_eq!(fixture.core.events().len(), events_before);

    fixture
        .core
        .suspend_agent(fixture.manager)
        .expect("manager should suspend");
    let events_before = fixture.core.events().len();
    assert_eq!(
        fixture
            .core
            .retire_orphaned_message(fixture.manager, fixture.authority, message),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(fixture.core.messages(), &[record]);
    assert_eq!(fixture.core.events().len(), events_before);

    assert_eq!(
        fixture.core.retire_orphaned_message(
            AgentId::new(77),
            fixture.authority,
            MessageId::new(77),
        ),
        Err(KernelError::AgentNotFound)
    );
}

#[test]
fn namespace_reference_blocks_orphaned_message_retirement() {
    let operations = OperationSet::empty()
        .with(Operation::Act)
        .with(Operation::Delegate);
    let mut fixture = setup(operations);
    let message = pending_message(&mut fixture, MessagePayload::empty());
    fixture
        .core
        .bind_namespace_entry(
            fixture.manager,
            fixture.authority,
            fixture.resource,
            NamespaceKey::new(1),
            NamespaceObject::Message(message),
        )
        .expect("message binding should fit");
    retire_target(&mut fixture);
    let record = fixture.core.messages()[0];
    let events_before = fixture.core.events().len();

    assert_eq!(
        fixture
            .core
            .retire_orphaned_message(fixture.manager, fixture.authority, message),
        Err(KernelError::MessageRetirementReferenced)
    );
    assert_eq!(fixture.core.messages(), &[record]);
    assert_eq!(fixture.core.events().len(), events_before);
}

#[test]
fn dense_removal_preserves_order_and_reuses_capacity_with_monotonic_ids() {
    let mut fixture = setup(OperationSet::only(Operation::Delegate));
    let first_recipient = AgentId::new(2);
    let last_recipient = AgentId::new(3);
    fixture.core.register_agent(first_recipient).unwrap();
    fixture.core.register_agent(last_recipient).unwrap();
    let first = fixture
        .core
        .send_message(
            fixture.manager,
            first_recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .unwrap();
    let orphaned = pending_message(&mut fixture, MessagePayload::empty());
    let last = fixture
        .core
        .send_message(
            fixture.manager,
            last_recipient,
            MessageKind::Response,
            MessagePayload::empty(),
        )
        .unwrap();
    assert_eq!([first.raw(), orphaned.raw(), last.raw()], [1, 2, 3]);
    retire_target(&mut fixture);

    fixture
        .core
        .retire_orphaned_message(fixture.manager, fixture.authority, orphaned)
        .expect("middle orphan should retire");
    assert_eq!(
        fixture
            .core
            .messages()
            .iter()
            .map(|record| record.id.raw())
            .collect::<Vec<_>>(),
        vec![1, 3]
    );

    let reused = fixture
        .core
        .send_message(
            fixture.manager,
            first_recipient,
            MessageKind::Fault,
            MessagePayload::empty(),
        )
        .expect("retired slot should be reusable");
    assert_eq!(reused, MessageId::new(4));
    assert_eq!(
        fixture
            .core
            .messages()
            .iter()
            .map(|record| record.id.raw())
            .collect::<Vec<_>>(),
        vec![1, 3, 4]
    );
}

#[test]
fn missing_message_and_event_exhaustion_leave_state_unchanged() {
    let mut fixture = setup(OperationSet::only(Operation::Delegate));
    let _message = pending_message(&mut fixture, MessagePayload::empty());
    retire_target(&mut fixture);
    let record = fixture.core.messages()[0];
    let events_before = fixture.core.events().len();
    assert_eq!(
        fixture.core.retire_orphaned_message(
            fixture.manager,
            fixture.authority,
            MessageId::new(99),
        ),
        Err(KernelError::MessageNotFound)
    );
    assert_eq!(fixture.core.messages(), &[record]);
    assert_eq!(fixture.core.events().len(), events_before);

    type FullEventCore = KernelCore<2, 1, 1, 5, 0, 0, 0, 0, 0, 0, 1>;
    let mut core = FullEventCore::new();
    let manager = AgentId::new(1);
    let target = AgentId::new(9);
    core.register_agent(manager).unwrap();
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = core
        .grant_capability(manager, resource, OperationSet::only(Operation::Delegate))
        .unwrap();
    core.register_managed_agent(manager, authority, resource, target)
        .unwrap();
    let message = core
        .send_message(
            manager,
            target,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .unwrap();
    core.retire_managed_agent(manager, authority, target)
        .unwrap();
    assert_eq!(core.events().len(), 5);
    let record = core.messages()[0];

    assert_eq!(
        core.retire_orphaned_message(manager, authority, message),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.messages(), &[record]);
    assert_eq!(core.events().len(), 5);
}
