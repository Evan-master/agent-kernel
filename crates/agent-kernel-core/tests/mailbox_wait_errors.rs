mod support;

use agent_kernel_core::{
    AgentExecutionState, KernelError, MessageKind, MessagePayload, TaskStatus,
};

use support::{running_recipient, MailboxWaitCore};

#[test]
fn mailbox_waiter_capacity_failure_leaves_receiver_running() {
    let mut core = MailboxWaitCore::<40, 2, 1, 0>::new();
    let flow = running_recipient(&mut core);
    let events_before = core.events().len();

    assert_eq!(
        core.receive_or_wait_message(
            flow.recipient,
            flow.recipient_capability,
            flow.recipient_task,
        ),
        Err(KernelError::WaiterStoreFull)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(
        core.execution_context(flow.recipient).unwrap().state,
        AgentExecutionState::Running
    );
    assert!(core.waiters().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn mailbox_wait_event_capacity_failure_leaves_receiver_running() {
    const EVENTS: usize = 40;
    let mut core = MailboxWaitCore::<EVENTS, 2, 1, 1>::new();
    let flow = running_recipient(&mut core);
    while core.events().len() < EVENTS {
        core.observe(flow.owner, flow.owner_capability, flow.resource)
            .expect("audit event should fill log");
    }

    assert_eq!(
        core.receive_or_wait_message(
            flow.recipient,
            flow.recipient_capability,
            flow.recipient_task,
        ),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert_eq!(
        core.execution_context(flow.recipient).unwrap().state,
        AgentExecutionState::Running
    );
    assert!(core.waiters().is_empty());
    assert_eq!(core.events().len(), EVENTS);
}

#[test]
fn wake_run_queue_full_leaves_message_and_waiter_unchanged() {
    let mut core = MailboxWaitCore::<40, 1, 1, 1>::new();
    let flow = running_recipient(&mut core);
    core.receive_or_wait_message(
        flow.recipient,
        flow.recipient_capability,
        flow.recipient_task,
    )
    .expect("recipient should wait");
    let events_before = core.events().len();

    assert_eq!(
        core.send_message(
            flow.sender,
            flow.recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        ),
        Err(KernelError::RunQueueFull)
    );
    assert!(core.messages().is_empty());
    assert!(core.waiters()[0].active);
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(
        core.execution_context(flow.recipient).unwrap().state,
        AgentExecutionState::Waiting
    );
    assert_eq!(core.run_queue()[0].task, flow.sender_task);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn wake_event_capacity_failure_is_atomic() {
    const EVENTS: usize = 40;
    let mut core = MailboxWaitCore::<EVENTS, 2, 1, 1>::new();
    let flow = running_recipient(&mut core);
    core.receive_or_wait_message(
        flow.recipient,
        flow.recipient_capability,
        flow.recipient_task,
    )
    .expect("recipient should wait");
    while core.events().len() + 1 < EVENTS {
        core.observe(flow.owner, flow.owner_capability, flow.resource)
            .expect("audit event should fill log");
    }
    let events_before = core.events().len();

    assert_eq!(
        core.send_message(
            flow.sender,
            flow.recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        ),
        Err(KernelError::EventLogFull)
    );
    assert!(core.messages().is_empty());
    assert!(core.waiters()[0].active);
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn wake_message_capacity_failure_is_atomic() {
    let mut core = MailboxWaitCore::<40, 2, 0, 1>::new();
    let flow = running_recipient(&mut core);
    core.receive_or_wait_message(
        flow.recipient,
        flow.recipient_capability,
        flow.recipient_task,
    )
    .expect("recipient should wait");
    let events_before = core.events().len();

    assert_eq!(
        core.send_message(
            flow.sender,
            flow.recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        ),
        Err(KernelError::MessageStoreFull)
    );
    assert!(core.waiters()[0].active);
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn mailbox_wait_requires_the_task_owned_capability() {
    let mut core = MailboxWaitCore::<40, 2, 1, 1>::new();
    let flow = running_recipient(&mut core);
    let events_before = core.events().len();

    assert_eq!(
        core.receive_or_wait_message(flow.recipient, flow.owner_capability, flow.recipient_task,),
        Err(KernelError::AgentMismatch)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Running);
    assert!(core.waiters().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn revoked_waiter_admission_rejects_send_without_partial_wake() {
    let mut core = MailboxWaitCore::<40, 2, 1, 1>::new();
    let flow = running_recipient(&mut core);
    core.receive_or_wait_message(
        flow.recipient,
        flow.recipient_capability,
        flow.recipient_task,
    )
    .expect("recipient should wait");
    core.revoke_capability(flow.recipient_capability)
        .expect("recipient authority should revoke");
    let events_before = core.events().len();

    assert_eq!(
        core.send_message(
            flow.sender,
            flow.recipient,
            MessageKind::Notify,
            MessagePayload::empty(),
        ),
        Err(KernelError::CapabilityRevoked)
    );
    assert!(core.messages().is_empty());
    assert!(core.waiters()[0].active);
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.run_queue()[0].task, flow.sender_task);
    assert_eq!(core.events().len(), events_before);
}
