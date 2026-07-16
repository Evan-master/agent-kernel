mod support;

use agent_kernel_core::{
    EventKind, MessageKind, MessagePayload, MessageReceiveOutcome, MessageStatus, SignalKey,
    TaskStatus, WaiterId, WaiterKind,
};

use support::{running_recipient, MailboxWaitCore};

#[test]
fn empty_receive_waits_with_owned_task_authority() {
    let mut core = MailboxWaitCore::<40, 2, 1, 1>::new();
    let flow = running_recipient(&mut core);

    let outcome = core
        .receive_or_wait_message(
            flow.recipient,
            flow.recipient_capability,
            flow.recipient_task,
        )
        .expect("empty mailbox should create a waiter");

    assert_eq!(outcome, MessageReceiveOutcome::Waiting(WaiterId::new(1)));
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.run_queue()[0].task, flow.sender_task);
    let waiter = core.waiters()[0];
    assert_eq!(waiter.kind, WaiterKind::Mailbox);
    assert_eq!(waiter.agent, flow.recipient);
    assert_eq!(waiter.task, flow.recipient_task);
    assert_eq!(waiter.resource, flow.resource);
    assert_eq!(waiter.signal.raw(), 0);
    assert!(waiter.active);
    let event = core.events().last().unwrap();
    assert_eq!(event.kind, EventKind::MessageWaitStarted);
    assert_eq!(event.agent, flow.recipient);
    assert_eq!(event.capability, Some(flow.recipient_capability));
    assert_eq!(event.task, Some(flow.recipient_task));
    assert_eq!(event.waiter, Some(waiter.id));
    assert_eq!(event.message, None);
}

#[test]
fn send_wakes_mailbox_waiter_behind_existing_runnable_task() {
    let mut core = MailboxWaitCore::<40, 2, 1, 1>::new();
    let flow = running_recipient(&mut core);
    core.receive_or_wait_message(
        flow.recipient,
        flow.recipient_capability,
        flow.recipient_task,
    )
    .expect("recipient should wait");

    let message = core
        .send_message(
            flow.sender,
            flow.recipient,
            MessageKind::Notify,
            MessagePayload {
                task: Some(flow.sender_task),
                ..MessagePayload::empty()
            },
        )
        .expect("send should wake recipient");

    assert_eq!(core.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(core.run_queue().len(), 2);
    assert_eq!(core.run_queue()[0].task, flow.sender_task);
    assert_eq!(core.run_queue()[1].task, flow.recipient_task);
    assert!(!core.waiters()[0].active);
    assert_eq!(core.messages()[0].id, message);
    assert_eq!(core.messages()[0].status, MessageStatus::Pending);
    let events = core.events();
    assert_eq!(events[events.len() - 2].kind, EventKind::MessageSent);
    let wake = events[events.len() - 1];
    assert_eq!(wake.kind, EventKind::MessageWaitWoken);
    assert_eq!(wake.agent, flow.sender);
    assert_eq!(wake.target_agent, Some(flow.recipient));
    assert_eq!(wake.task, Some(flow.recipient_task));
    assert_eq!(wake.waiter, Some(WaiterId::new(1)));
    assert_eq!(wake.message, Some(message));
}

#[test]
fn pending_message_is_received_without_allocating_waiter() {
    let mut core = MailboxWaitCore::<40, 2, 1, 1>::new();
    let flow = running_recipient(&mut core);
    let message = core
        .send_message(
            flow.sender,
            flow.recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message should send");

    let outcome = core
        .receive_or_wait_message(
            flow.recipient,
            flow.recipient_capability,
            flow.recipient_task,
        )
        .expect("pending message should receive");

    assert_eq!(outcome, MessageReceiveOutcome::Received(message));
    assert!(core.waiters().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(core.messages()[0].status, MessageStatus::Received);
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::MessageReceived
    );
}

#[test]
fn message_send_does_not_wake_a_signal_waiter() {
    let mut core = MailboxWaitCore::<40, 2, 1, 1>::new();
    let flow = running_recipient(&mut core);
    core.wait_task(
        flow.recipient,
        flow.recipient_capability,
        flow.recipient_task,
        flow.resource,
        SignalKey::new(9),
    )
    .expect("recipient should wait for a signal");
    let events_before = core.events().len();

    core.send_message(
        flow.sender,
        flow.recipient,
        MessageKind::Notify,
        MessagePayload::empty(),
    )
    .expect("message should not consume a signal waiter");

    assert_eq!(core.events().len(), events_before + 1);
    assert_eq!(core.events().last().unwrap().kind, EventKind::MessageSent);
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.waiters()[0].kind, WaiterKind::Signal);
    assert!(core.waiters()[0].active);
    assert_eq!(core.run_queue()[0].task, flow.sender_task);
}
