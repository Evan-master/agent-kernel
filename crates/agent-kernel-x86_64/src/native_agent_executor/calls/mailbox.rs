//! Mailbox Agent Call handlers for the native runtime loop.

use agent_kernel_core::{
    AgentId, EventKind, MessageId, MessageKind, MessagePayload, MessageReceiveOutcome,
    MessageStatus,
};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu, WaitingAgentCallCpu},
    X86BootedKernel,
};

pub(super) enum ReceiveDisposition {
    Continue(ResumableAgentCpu),
    Waiting(WaitingAgentCallCpu),
}

pub(super) fn send(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    recipient: AgentId,
    kind: MessageKind,
    payload: MessagePayload,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event_start = booted.kernel().events().len();
    let message = booted
        .kernel_mut()
        .sys_send_message(context.agent(), recipient, kind, payload)
        .ok()?;
    let record = message_record(booted, message)?;
    let events = booted.kernel().events().get(event_start..)?;
    if record.sender != context.agent()
        || record.recipient != recipient
        || record.kind != kind
        || record.payload != payload
        || record.status != MessageStatus::Pending
        || !matches!(events, [sent] | [sent, _]
            if sent.kind == EventKind::MessageSent
                && sent.agent == context.agent()
                && sent.target_agent == Some(recipient)
                && sent.message == Some(message))
        || events.get(1).is_some_and(|wake| {
            wake.kind != EventKind::MessageWaitWoken
                || wake.agent != context.agent()
                || wake.target_agent != Some(recipient)
                || wake.message != Some(message)
        })
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_message_send(message)
}

pub(super) fn receive_or_wait(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
) -> Option<ReceiveDisposition> {
    let context = authenticated_context(&pending)?;
    match booted
        .kernel_mut()
        .sys_receive_or_wait_message(context.agent(), context.capability(), context.task())
        .ok()?
    {
        MessageReceiveOutcome::Received(message) => {
            let record = validate_received(booted, context.agent(), message)?;
            Some(ReceiveDisposition::Continue(
                pending.acknowledge_message_receive(record)?,
            ))
        }
        MessageReceiveOutcome::Waiting(waiter) => {
            let event = booted.kernel().events().last()?;
            if event.kind != EventKind::MessageWaitStarted
                || event.agent != context.agent()
                || event.capability != Some(context.capability())
                || event.task != Some(context.task())
                || event.waiter != Some(waiter)
                || !state::waiting(booted, context)
            {
                return None;
            }
            Some(ReceiveDisposition::Waiting(pending.wait(waiter)?))
        }
    }
}

pub(super) fn resume_waiting(
    booted: &mut X86BootedKernel,
    waiting: WaitingAgentCallCpu,
) -> Option<ResumableAgentCpu> {
    let context = waiting.context();
    if waiting.waiter().raw() == 0 || !state::running(booted, context) {
        return None;
    }
    let MessageReceiveOutcome::Received(message) = booted
        .kernel_mut()
        .sys_receive_or_wait_message(context.agent(), context.capability(), context.task())
        .ok()?
    else {
        return None;
    };
    let record = validate_received(booted, context.agent(), message)?;
    waiting.acknowledge_message_receive(record)
}

pub(super) fn acknowledge(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    message: MessageId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let event = booted
        .kernel_mut()
        .sys_acknowledge_message(context.agent(), message)
        .ok()?;
    let record = message_record(booted, message)?;
    if event.kind != EventKind::MessageAcknowledged
        || event.agent != context.agent()
        || event.message != Some(message)
        || record.recipient != context.agent()
        || record.status != MessageStatus::Acknowledged
        || !state::running(booted, context)
    {
        return None;
    }
    pending.acknowledge_message()
}

pub(super) fn retire(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    message: MessageId,
) -> Option<ResumableAgentCpu> {
    let context = authenticated_context(&pending)?;
    let retirement = booted
        .kernel_mut()
        .sys_retire_message(context.agent(), message)
        .ok()?;
    let record = retirement.record();
    let event = booted.kernel().events().last()?;
    if retirement.message() != message
        || record.recipient != context.agent()
        || record.status != MessageStatus::Acknowledged
        || message_record(booted, message).is_some()
        || event.kind != EventKind::MessageRetired
        || event.agent != context.agent()
        || event.target_agent != Some(record.sender)
        || event.message != Some(message)
        || !state::running(booted, context)
    {
        return None;
    }
    crate::serial_write_line("AGENT_KERNEL_AGENT_CALL_MESSAGE_RETIREMENT_OK");
    pending.acknowledge_message_retirement(message)
}

fn validate_received(
    booted: &X86BootedKernel,
    recipient: AgentId,
    message: MessageId,
) -> Option<agent_kernel_core::MessageRecord> {
    let record = message_record(booted, message)?;
    let event = booted.kernel().events().last()?;
    (record.recipient == recipient
        && record.status == MessageStatus::Received
        && event.kind == EventKind::MessageReceived
        && event.agent == recipient
        && event.target_agent == Some(record.sender)
        && event.message == Some(message))
    .then_some(record)
}

fn message_record(
    booted: &X86BootedKernel,
    message: MessageId,
) -> Option<agent_kernel_core::MessageRecord> {
    booted
        .kernel()
        .messages()
        .iter()
        .copied()
        .find(|record| record.id == message)
}

fn authenticated_context(
    pending: &PendingAgentCallCpu,
) -> Option<agent_kernel_x86_64::agent_call::AgentCallContext> {
    pending.authenticated_request()?;
    Some(pending.context())
}
