//! Mailbox semantic transitions bound to physical Worker CPU evidence.
//!
//! Each helper calls only public facade methods, validates the exact message
//! event and record, and proves the matching scheduler transition before
//! constructing a reply-capable CPU token.

mod state;

use agent_kernel_core::{
    EventKind, MessageKind, MessagePayload, MessageRecord, MessageStatus, WaiterId,
};

use super::WorkerTask;
use crate::{
    agent_cpu::{
        AcknowledgedMessageAcknowledgementCpu, AcknowledgedMessageReceiveCpu,
        AcknowledgedMessageSendCpu, RequestedMessageAcknowledgementCpu, RequestedMessageSendCpu,
        WaitingMessageReceiveCpu,
    },
    X86BootedKernel,
};

pub(super) fn send(
    booted: &mut X86BootedKernel,
    sender: WorkerTask,
    recipient: WorkerTask,
    waiter: WaiterId,
    cpu: RequestedMessageSendCpu,
) -> Option<AcknowledgedMessageSendCpu> {
    let expected_payload = MessagePayload {
        task: Some(sender.task),
        ..MessagePayload::empty()
    };
    if cpu.call_count() != 3
        || cpu.address_space_switch_count() != 6
        || sender.call_context() != Some(cpu.context())
        || cpu.recipient() != recipient.agent
        || cpu.kind() != MessageKind::Notify
        || cpu.payload() != expected_payload
        || !state::sender_before_send_valid(booted, sender, recipient, waiter)
    {
        return None;
    }
    let message = booted
        .kernel_mut()
        .sys_send_message(
            sender.agent,
            recipient.agent,
            MessageKind::Notify,
            expected_payload,
        )
        .ok()?;
    let record = find_message(booted, message)?;
    let events = booted.kernel().events();
    let sent = events.get(events.len().checked_sub(2)?)?;
    let wake = events.last()?;
    if record.sender != sender.agent
        || record.recipient != recipient.agent
        || record.kind != MessageKind::Notify
        || record.payload != expected_payload
        || record.status != MessageStatus::Pending
        || sent.kind != EventKind::MessageSent
        || sent.agent != sender.agent
        || sent.target_agent != Some(recipient.agent)
        || sent.message != Some(message)
        || wake.kind != EventKind::MessageWaitWoken
        || wake.agent != sender.agent
        || wake.capability.is_some()
        || wake.target_agent != Some(recipient.agent)
        || wake.task != Some(recipient.task)
        || wake.waiter != Some(waiter)
        || wake.message != Some(message)
        || !state::sender_after_wake_valid(booted, sender, recipient, waiter)
    {
        return None;
    }
    cpu.acknowledge(message)
}

pub(super) fn receive(
    booted: &mut X86BootedKernel,
    recipient: WorkerTask,
    sender: WorkerTask,
    waiter: WaiterId,
    cpu: WaitingMessageReceiveCpu,
) -> Option<AcknowledgedMessageReceiveCpu> {
    if cpu.call_count() != 2
        || cpu.address_space_switch_count() != 4
        || recipient.call_context() != Some(cpu.context())
        || cpu.waiter() != waiter
        || !cpu.agent_call_is_released()
        || !state::receiver_running_valid(booted, recipient, sender)
    {
        return None;
    }
    let message = booted
        .kernel_mut()
        .sys_receive_message(recipient.agent)
        .ok()?;
    let record = find_message(booted, message)?;
    let event = booted.kernel().events().last()?;
    if !expected_record(record, sender, recipient, MessageStatus::Received)
        || event.kind != EventKind::MessageReceived
        || event.agent != recipient.agent
        || event.target_agent != Some(sender.agent)
        || event.message != Some(message)
        || !state::receiver_running_valid(booted, recipient, sender)
    {
        return None;
    }
    cpu.acknowledge(record)
}

pub(super) fn acknowledge(
    booted: &mut X86BootedKernel,
    recipient: WorkerTask,
    sender: WorkerTask,
    cpu: RequestedMessageAcknowledgementCpu,
) -> Option<AcknowledgedMessageAcknowledgementCpu> {
    if cpu.call_count() != 3
        || cpu.address_space_switch_count() != 6
        || recipient.call_context() != Some(cpu.context())
        || !state::receiver_running_valid(booted, recipient, sender)
    {
        return None;
    }
    let message = cpu.message();
    let event = booted
        .kernel_mut()
        .sys_acknowledge_message(recipient.agent, message)
        .ok()?;
    let record = find_message(booted, message)?;
    if !expected_record(record, sender, recipient, MessageStatus::Acknowledged)
        || event.kind != EventKind::MessageAcknowledged
        || event.agent != recipient.agent
        || event.target_agent != Some(sender.agent)
        || event.message != Some(message)
        || !state::receiver_running_valid(booted, recipient, sender)
    {
        return None;
    }
    cpu.acknowledge()
}

fn find_message(
    booted: &X86BootedKernel,
    message: agent_kernel_core::MessageId,
) -> Option<MessageRecord> {
    booted
        .kernel()
        .messages()
        .iter()
        .copied()
        .find(|record| record.id == message)
}

fn expected_record(
    record: MessageRecord,
    sender: WorkerTask,
    recipient: WorkerTask,
    status: MessageStatus,
) -> bool {
    record.sender == sender.agent
        && record.recipient == recipient.agent
        && record.kind == MessageKind::Notify
        && record.payload
            == MessagePayload {
                task: Some(sender.task),
                ..MessagePayload::empty()
            }
        && record.status == status
}
