use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{AgentId, EventKind, MessageKind, MessagePayload, MessageStatus, TaskId};

type MailboxBoot = BootedKernel<2, 1, 2, 16, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn boot_message_capacity_is_explicit_and_usable() {
    let mut booted = MailboxBoot::boot(BootConfig::default()).unwrap();
    let sender = booted.report().bootstrap_agent;
    let recipient = AgentId::new(2);
    booted.kernel_mut().sys_register_agent(recipient).unwrap();

    let message = booted
        .kernel_mut()
        .sys_send_message(
            sender,
            recipient,
            MessageKind::Notify,
            MessagePayload {
                task: Some(TaskId::new(9)),
                ..MessagePayload::empty()
            },
        )
        .unwrap();

    let record = booted.kernel().messages()[0];
    assert_eq!(record.id, message);
    assert_eq!(record.status, MessageStatus::Pending);
    assert_eq!(record.payload.task, Some(TaskId::new(9)));
    assert_eq!(
        booted.kernel().events().last().unwrap().kind,
        EventKind::MessageSent
    );
}
