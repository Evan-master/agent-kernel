use agent_kernel_core::{
    AgentId, EventKind, KernelCore, MessageId, MessageKind, MessagePayload, MessageStatus,
};

type TestCore = KernelCore<3, 1, 1, 16, 0, 0, 0, 0, 0, 0, 4>;

fn register_agents(core: &mut TestCore, agents: &[AgentId]) {
    for agent in agents {
        core.register_agent(*agent)
            .expect("agent registration should fit");
    }
}

#[test]
fn send_message_records_pending_message_and_event() {
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

    assert_eq!(message, MessageId::new(1));
    assert_eq!(core.messages().len(), 1);
    assert_eq!(core.messages()[0].id, message);
    assert_eq!(core.messages()[0].sender, sender);
    assert_eq!(core.messages()[0].recipient, recipient);
    assert_eq!(core.messages()[0].kind, MessageKind::Notify);
    assert_eq!(core.messages()[0].status, MessageStatus::Pending);
    assert_eq!(core.events()[2].kind, EventKind::MessageSent);
    assert_eq!(core.events()[2].agent, sender);
    assert_eq!(core.events()[2].target_agent, Some(recipient));
    assert_eq!(core.events()[2].message, Some(message));
}

#[test]
fn receive_message_delivers_oldest_pending_message_for_recipient() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    register_agents(&mut core, &[sender, recipient]);
    let first = core
        .send_message(
            sender,
            recipient,
            MessageKind::Request,
            MessagePayload::empty(),
        )
        .expect("first message should fit");
    let second = core
        .send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("second message should fit");

    let received = core
        .receive_message(recipient)
        .expect("pending message should be delivered");

    assert_eq!(received, first);
    assert_eq!(core.messages()[0].status, MessageStatus::Received);
    assert_eq!(core.messages()[1].id, second);
    assert_eq!(core.messages()[1].status, MessageStatus::Pending);
    assert_eq!(core.events()[4].kind, EventKind::MessageReceived);
    assert_eq!(core.events()[4].agent, recipient);
    assert_eq!(core.events()[4].target_agent, Some(sender));
    assert_eq!(core.events()[4].message, Some(first));
}

#[test]
fn acknowledge_message_closes_received_message_and_records_event() {
    let mut core = TestCore::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    register_agents(&mut core, &[sender, recipient]);
    let message = core
        .send_message(
            sender,
            recipient,
            MessageKind::Response,
            MessagePayload::empty(),
        )
        .expect("message should fit");
    core.receive_message(recipient)
        .expect("message should be received before acknowledgement");

    let event = core
        .acknowledge_message(recipient, message)
        .expect("received message should acknowledge");

    assert_eq!(core.messages()[0].status, MessageStatus::Acknowledged);
    assert_eq!(event.kind, EventKind::MessageAcknowledged);
    assert_eq!(event.agent, recipient);
    assert_eq!(event.target_agent, Some(sender));
    assert_eq!(event.message, Some(message));
}
