use agent_kernel_core::{
    AgentId, KernelCore, KernelError, MessageKind, MessagePayload, MessageStatus,
};

type TestCore = KernelCore<3, 1, 1, 16, 0, 0, 0, 0, 0, 0, 4>;

fn register_agents(core: &mut TestCore, agents: &[AgentId]) {
    for agent in agents {
        core.register_agent(*agent)
            .expect("agent registration should fit");
    }
}

#[test]
fn receive_empty_mailbox_returns_mailbox_empty_without_event() {
    let mut core = TestCore::new();
    let recipient = AgentId::new(2);
    core.register_agent(recipient)
        .expect("agent registration should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.receive_message(recipient),
        Err(KernelError::MailboxEmpty)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn send_message_rejects_inactive_agents_without_mutation() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let suspended_recipient = AgentId::new(2);
    let retired_recipient = AgentId::new(3);
    register_agents(&mut core, &[sender, suspended_recipient, retired_recipient]);
    core.suspend_agent(suspended_recipient)
        .expect("recipient should suspend");
    core.retire_agent(retired_recipient)
        .expect("recipient should retire");
    let events_before = core.events().len();

    assert_eq!(
        core.send_message(
            AgentId::new(99),
            suspended_recipient,
            MessageKind::Notify,
            MessagePayload::empty()
        ),
        Err(KernelError::AgentNotFound)
    );
    assert_eq!(
        core.send_message(
            sender,
            suspended_recipient,
            MessageKind::Notify,
            MessagePayload::empty()
        ),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(
        core.send_message(
            sender,
            retired_recipient,
            MessageKind::Notify,
            MessagePayload::empty()
        ),
        Err(KernelError::AgentRetired)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn acknowledge_message_rejects_wrong_recipient_without_mutation() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    let wrong_recipient = AgentId::new(3);
    register_agents(&mut core, &[sender, recipient, wrong_recipient]);
    let message = core
        .send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message should fit");
    core.receive_message(recipient)
        .expect("message should be received");
    let events_before = core.events().len();

    assert_eq!(
        core.acknowledge_message(wrong_recipient, message),
        Err(KernelError::MessageAgentMismatch)
    );
    assert_eq!(core.messages()[0].status, MessageStatus::Received);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn acknowledge_message_rejects_pending_message_without_mutation() {
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
    let events_before = core.events().len();

    assert_eq!(
        core.acknowledge_message(recipient, message),
        Err(KernelError::MessageStatusMismatch)
    );
    assert_eq!(core.messages()[0].status, MessageStatus::Pending);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn send_message_store_full_leaves_events_unchanged() {
    let mut core = KernelCore::<2, 1, 1, 8, 0, 0, 0, 0, 0, 0, 1>::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    core.register_agent(sender).expect("sender should fit");
    core.register_agent(recipient)
        .expect("recipient should fit");
    core.send_message(
        sender,
        recipient,
        MessageKind::Notify,
        MessagePayload::empty(),
    )
    .expect("first message should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty()
        ),
        Err(KernelError::MessageStoreFull)
    );
    assert_eq!(core.messages().len(), 1);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn send_message_event_log_full_leaves_messages_unchanged() {
    let mut core = KernelCore::<2, 1, 1, 2, 0, 0, 0, 0, 0, 0, 1>::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    core.register_agent(sender)
        .expect("first registration should fit");
    core.register_agent(recipient)
        .expect("second registration should consume final event slot");

    assert_eq!(
        core.send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty()
        ),
        Err(KernelError::EventLogFull)
    );
    assert!(core.messages().is_empty());
    assert_eq!(core.events().len(), 2);
}
