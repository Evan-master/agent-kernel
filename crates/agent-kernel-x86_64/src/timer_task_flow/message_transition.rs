//! Mailbox semantic transitions bound to physical Worker CPU evidence.
//!
//! Each helper calls only public facade methods, validates the exact message
//! event and record, and proves that mailbox mutation leaves scheduler state
//! unchanged before constructing a reply-capable CPU token.

use agent_kernel_core::{
    AgentExecutionState, EventKind, MessageKind, MessagePayload, MessageRecord, MessageStatus,
    RunQueueEntry, TaskStatus,
};

use super::WorkerTask;
use crate::{
    agent_cpu::{
        AcknowledgedMessageAcknowledgementCpu, AcknowledgedMessageReceiveCpu,
        AcknowledgedMessageSendCpu, RequestedMessageAcknowledgementCpu, RequestedMessageReceiveCpu,
        RequestedMessageSendCpu,
    },
    X86BootedKernel,
};

pub(super) fn send(
    booted: &mut X86BootedKernel,
    sender: WorkerTask,
    recipient: WorkerTask,
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
    let event = booted.kernel().events().last()?;
    if record.sender != sender.agent
        || record.recipient != recipient.agent
        || record.kind != MessageKind::Notify
        || record.payload != expected_payload
        || record.status != MessageStatus::Pending
        || event.kind != EventKind::MessageSent
        || event.agent != sender.agent
        || event.target_agent != Some(recipient.agent)
        || event.message != Some(message)
        || !sender_state_valid(booted, sender, recipient)
    {
        return None;
    }
    cpu.acknowledge(message)
}

pub(super) fn receive(
    booted: &mut X86BootedKernel,
    recipient: WorkerTask,
    sender: WorkerTask,
    cpu: RequestedMessageReceiveCpu,
) -> Option<AcknowledgedMessageReceiveCpu> {
    if cpu.call_count() != 2
        || cpu.address_space_switch_count() != 4
        || recipient.call_context() != Some(cpu.context())
        || !receiver_state_valid(booted, recipient, sender)
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
        || !receiver_state_valid(booted, recipient, sender)
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
        || !receiver_state_valid(booted, recipient, sender)
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
        || !receiver_state_valid(booted, recipient, sender)
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

fn sender_state_valid(booted: &X86BootedKernel, sender: WorkerTask, recipient: WorkerTask) -> bool {
    let kernel = booted.kernel();
    let sender_task = kernel.tasks().iter().find(|task| task.id == sender.task);
    let recipient_task = kernel.tasks().iter().find(|task| task.id == recipient.task);
    let sender_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == sender.agent);
    let recipient_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == recipient.agent);
    matches!(sender_task, Some(task) if task.status == TaskStatus::Running
        && task.result == Some(sender.result) && task.run_ticks == 1)
        && matches!(recipient_task, Some(task) if task.status == TaskStatus::Accepted
            && task.result.is_none() && task.run_ticks == 1)
        && matches!(sender_context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(sender.task))
        && matches!(recipient_context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none())
        && kernel.run_queue()
            == [RunQueueEntry {
                task: recipient.task,
                agent: recipient.agent,
            }]
}

fn receiver_state_valid(
    booted: &X86BootedKernel,
    recipient: WorkerTask,
    sender: WorkerTask,
) -> bool {
    let kernel = booted.kernel();
    let recipient_task = kernel.tasks().iter().find(|task| task.id == recipient.task);
    let sender_task = kernel.tasks().iter().find(|task| task.id == sender.task);
    let recipient_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == recipient.agent);
    let sender_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == sender.agent);
    matches!(recipient_task, Some(task) if task.status == TaskStatus::Running
        && task.result.is_none() && task.run_ticks == 1)
        && matches!(sender_task, Some(task) if task.status == TaskStatus::Completed
            && task.result == Some(sender.result) && task.run_ticks == 1)
        && matches!(recipient_context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(recipient.task))
        && matches!(sender_context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none())
        && kernel.run_queue().is_empty()
}
