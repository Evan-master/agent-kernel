use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, KernelError, MessageId, MessageKind, MessagePayload, MessageStatus,
};

type TestKernel = AgentKernel<2, 1, 1, 8, 0, 0, 0, 0, 0, 0, 2>;

#[test]
fn mailbox_syscalls_send_receive_acknowledge_and_expose_messages() {
    let mut kernel = TestKernel::new();
    let sender = AgentId::new(1);
    let recipient = AgentId::new(2);
    kernel
        .sys_register_agent(sender)
        .expect("sender registration should fit");
    kernel
        .sys_register_agent(recipient)
        .expect("recipient registration should fit");

    let message = kernel
        .sys_send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message should fit");
    let received = kernel
        .sys_receive_message(recipient)
        .expect("message should deliver");
    let event = kernel
        .sys_acknowledge_message(recipient, message)
        .expect("received message should acknowledge");

    assert_eq!(message, MessageId::new(1));
    assert_eq!(received, message);
    assert_eq!(kernel.messages()[0].status, MessageStatus::Acknowledged);
    assert_eq!(kernel.events()[2].kind, EventKind::MessageSent);
    assert_eq!(kernel.events()[3].kind, EventKind::MessageReceived);
    assert_eq!(event.kind, EventKind::MessageAcknowledged);
    assert_eq!(
        kernel.sys_receive_message(recipient),
        Err(KernelError::MailboxEmpty)
    );
}
