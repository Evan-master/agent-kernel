use agent_kernel_core::{
    ActionId, AgentId, CapabilityId, EventKind, FaultId, IntentId, KernelCore, KernelError,
    MessageId, MessageKind, MessagePayload, MessageStatus, NamespaceKey, NamespaceObject,
    Operation, OperationSet, ResourceId, ResourceKind, TaskId,
};

type TestCore = KernelCore<4, 2, 2, 40, 0, 0, 0, 0, 0, 0, 3, 0, 2>;

fn register_agents(core: &mut TestCore, agents: &[AgentId]) {
    for agent in agents {
        core.register_agent(*agent)
            .expect("agent registration should fit");
    }
}

fn send_and_acknowledge(core: &mut TestCore, sender: AgentId, recipient: AgentId) -> MessageId {
    let message = core
        .send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message should fit");
    assert_eq!(
        core.receive_message(recipient),
        Ok(message),
        "recipient should receive the message"
    );
    core.acknowledge_message(recipient, message)
        .expect("received message should acknowledge");
    message
}

#[test]
fn recipient_retires_acknowledged_message_and_records_receipt_event() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    register_agents(&mut core, &[sender, recipient]);
    let payload = MessagePayload {
        resource: Some(ResourceId::new(11)),
        capability: Some(CapabilityId::new(12)),
        intent: Some(IntentId::new(13)),
        task: Some(TaskId::new(14)),
        action: Some(ActionId::new(15)),
        fault: Some(FaultId::new(16)),
    };
    let message = core
        .send_message(sender, recipient, MessageKind::Response, payload)
        .expect("message should fit");
    core.receive_message(recipient)
        .expect("message should receive");
    core.acknowledge_message(recipient, message)
        .expect("message should acknowledge");

    let retirement = core
        .retire_message(recipient, message)
        .expect("recipient should retire acknowledged message");

    assert_eq!(retirement.message(), message);
    assert_eq!(retirement.record().sender, sender);
    assert_eq!(retirement.record().recipient, recipient);
    assert_eq!(retirement.record().status, MessageStatus::Acknowledged);
    assert!(core.messages().is_empty());
    let event = core.events().last().expect("retirement should emit event");
    assert_eq!(event.kind, EventKind::MessageRetired);
    assert_eq!(event.agent, recipient);
    assert_eq!(event.target_agent, Some(sender));
    assert_eq!(event.message, Some(message));
    assert_eq!(event.message_kind, Some(MessageKind::Response));
    assert_eq!(event.resource, payload.resource);
    assert_eq!(event.capability, payload.capability);
    assert_eq!(event.intent, payload.intent);
    assert_eq!(event.task, payload.task);
    assert_eq!(event.action, payload.action);
    assert_eq!(event.fault, payload.fault);
}

#[test]
fn retirement_preserves_dense_order_and_reuses_slot_with_monotonic_id() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let first_recipient = AgentId::new(2);
    let second_recipient = AgentId::new(3);
    register_agents(&mut core, &[sender, first_recipient, second_recipient]);
    let first = send_and_acknowledge(&mut core, sender, first_recipient);
    let second = send_and_acknowledge(&mut core, sender, second_recipient);
    let third = core
        .send_message(
            sender,
            first_recipient,
            MessageKind::Request,
            MessagePayload::empty(),
        )
        .expect("third message should fill the store");

    core.retire_message(second_recipient, second)
        .expect("middle message should retire");
    let fourth = core
        .send_message(
            sender,
            second_recipient,
            MessageKind::Response,
            MessagePayload::empty(),
        )
        .expect("retired slot should be reusable");

    assert_eq!(first, MessageId::new(1));
    assert_eq!(third, MessageId::new(3));
    assert_eq!(fourth, MessageId::new(4));
    assert_eq!(
        core.messages()
            .iter()
            .map(|record| record.id)
            .collect::<std::vec::Vec<_>>(),
        [first, third, fourth]
    );
    assert_eq!(core.receive_message(second_recipient), Ok(fourth));
}

#[test]
fn pending_and_received_messages_cannot_retire() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    register_agents(&mut core, &[sender, recipient]);
    let message = core
        .send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message should fit");
    let pending_events = core.events().len();

    assert_eq!(
        core.retire_message(recipient, message),
        Err(KernelError::MessageStatusMismatch)
    );
    assert_eq!(core.messages()[0].status, MessageStatus::Pending);
    assert_eq!(core.events().len(), pending_events);

    core.receive_message(recipient)
        .expect("message should become received");
    let received_events = core.events().len();
    assert_eq!(
        core.retire_message(recipient, message),
        Err(KernelError::MessageStatusMismatch)
    );
    assert_eq!(core.messages()[0].status, MessageStatus::Received);
    assert_eq!(core.events().len(), received_events);
}

#[test]
fn foreign_recipient_cannot_retire_message() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    let foreign = AgentId::new(3);
    register_agents(&mut core, &[sender, recipient, foreign]);
    let message = send_and_acknowledge(&mut core, sender, recipient);
    let messages_before = core.messages().to_vec();
    let events_before = core.events().len();

    assert_eq!(
        core.retire_message(foreign, message),
        Err(KernelError::MessageAgentMismatch)
    );
    assert_eq!(core.messages(), messages_before);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn inactive_and_unknown_callers_fail_before_message_lookup() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let suspended = AgentId::new(2);
    let retired = AgentId::new(3);
    register_agents(&mut core, &[sender, suspended, retired]);
    core.suspend_agent(suspended).expect("agent should suspend");
    core.retire_agent(retired).expect("agent should retire");
    let events_before = core.events().len();

    assert_eq!(
        core.retire_message(AgentId::new(99), MessageId::new(99)),
        Err(KernelError::AgentNotFound)
    );
    assert_eq!(
        core.retire_message(suspended, MessageId::new(99)),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(
        core.retire_message(retired, MessageId::new(99)),
        Err(KernelError::AgentRetired)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn active_recipient_cannot_retire_missing_message() {
    let mut core = TestCore::new();
    let recipient = AgentId::new(1);
    core.register_agent(recipient)
        .expect("recipient registration should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.retire_message(recipient, MessageId::new(99)),
        Err(KernelError::MessageNotFound)
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn namespace_reference_blocks_message_retirement() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    register_agents(&mut core, &[sender, recipient]);
    let message = send_and_acknowledge(&mut core, sender, recipient);
    let namespace = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("namespace resource should fit");
    let authority = core
        .grant_capability(
            recipient,
            namespace,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .expect("namespace capability should fit");
    core.bind_namespace_entry(
        recipient,
        authority,
        namespace,
        NamespaceKey::new(7),
        NamespaceObject::Message(message),
    )
    .expect("message namespace entry should bind");
    let messages_before = core.messages().to_vec();
    let events_before = core.events().len();

    assert_eq!(
        core.retire_message(recipient, message),
        Err(KernelError::MessageRetirementReferenced)
    );
    assert_eq!(core.messages(), messages_before);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn event_exhaustion_leaves_acknowledged_message_unchanged() {
    let mut core = KernelCore::<2, 0, 0, 5, 0, 0, 0, 0, 0, 0, 1>::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    core.register_agent(sender).expect("sender should fit");
    core.register_agent(recipient)
        .expect("recipient should fit");
    let message = core
        .send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message should fit");
    core.receive_message(recipient)
        .expect("message should receive");
    core.acknowledge_message(recipient, message)
        .expect("message should acknowledge");

    assert_eq!(
        core.retire_message(recipient, message),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.messages().len(), 1);
    assert_eq!(core.messages()[0].id, message);
    assert_eq!(core.messages()[0].status, MessageStatus::Acknowledged);
    assert_eq!(core.events().len(), 5);
}
