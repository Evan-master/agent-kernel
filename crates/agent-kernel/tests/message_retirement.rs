use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, MessageId, MessageKind, MessagePayload, MessageStatus,
};

type TestKernel = AgentKernel<2, 0, 0, 16, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn facade_retires_acknowledged_message_and_reuses_capacity() {
    let mut kernel = TestKernel::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    kernel
        .sys_register_agent(sender)
        .expect("sender registration should fit");
    kernel
        .sys_register_agent(recipient)
        .expect("recipient registration should fit");
    let first = kernel
        .sys_send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("first message should fit");
    kernel
        .sys_receive_message(recipient)
        .expect("first message should receive");
    kernel
        .sys_acknowledge_message(recipient, first)
        .expect("first message should acknowledge");

    let retirement = kernel
        .sys_retire_message(recipient, first)
        .expect("recipient should retire first message");
    let second = kernel
        .sys_send_message(
            sender,
            recipient,
            MessageKind::Response,
            MessagePayload::empty(),
        )
        .expect("retired slot should be reusable");

    assert_eq!(retirement.message(), first);
    assert_eq!(retirement.record().status, MessageStatus::Acknowledged);
    assert_eq!(second, MessageId::new(2));
    assert_eq!(kernel.messages().len(), 1);
    assert_eq!(kernel.messages()[0].id, second);
    assert_eq!(
        kernel.events()[5].kind,
        EventKind::MessageRetired,
        "facade should expose retirement evidence"
    );
}
