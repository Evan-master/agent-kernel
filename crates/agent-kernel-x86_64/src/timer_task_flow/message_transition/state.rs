//! Scheduler and waiter invariants around x86 mailbox message transitions.
//!
//! This x86 boot child reads public kernel state only. It proves the retained
//! receiver waiter, sender wake, and receiver redispatch states without owning
//! CPU frames or performing semantic mutation.

use agent_kernel_core::{AgentExecutionState, RunQueueEntry, TaskStatus, WaiterId, WaiterKind};

use super::super::WorkerTask;
use crate::X86BootedKernel;

pub(super) fn sender_before_send_valid(
    booted: &X86BootedKernel,
    sender: WorkerTask,
    recipient: WorkerTask,
    waiter: WaiterId,
) -> bool {
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
    let waiter = kernel.waiters().iter().find(|record| record.id == waiter);
    matches!(sender_task, Some(task) if task.status == TaskStatus::Running
        && task.result == Some(sender.result) && task.run_ticks == 1)
        && matches!(recipient_task, Some(task) if task.status == TaskStatus::Waiting
            && task.result.is_none() && task.run_ticks == 1)
        && matches!(sender_context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(sender.task))
        && matches!(recipient_context, Some(context) if context.state == AgentExecutionState::Waiting
            && context.task == Some(recipient.task))
        && matches!(waiter, Some(record) if record.active
            && record.kind == WaiterKind::Mailbox
            && record.agent == recipient.agent
            && record.task == recipient.task)
        && kernel.messages().is_empty()
        && kernel.run_queue().is_empty()
}

pub(super) fn sender_after_wake_valid(
    booted: &X86BootedKernel,
    sender: WorkerTask,
    recipient: WorkerTask,
    waiter: WaiterId,
) -> bool {
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
    let waiter = kernel.waiters().iter().find(|record| record.id == waiter);
    matches!(sender_task, Some(task) if task.status == TaskStatus::Running
        && task.result == Some(sender.result) && task.run_ticks == 1)
        && matches!(recipient_task, Some(task) if task.status == TaskStatus::Accepted
            && task.result.is_none() && task.run_ticks == 1)
        && matches!(sender_context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(sender.task))
        && matches!(recipient_context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none())
        && matches!(waiter, Some(record) if !record.active
            && record.kind == WaiterKind::Mailbox
            && record.agent == recipient.agent
            && record.task == recipient.task)
        && kernel.run_queue()
            == [RunQueueEntry {
                task: recipient.task,
                agent: recipient.agent,
            }]
}

pub(super) fn receiver_running_valid(
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
    let waiter = kernel.waiters().first();
    matches!(recipient_task, Some(task) if task.status == TaskStatus::Running
        && task.result.is_none() && task.run_ticks == 1)
        && matches!(sender_task, Some(task) if task.status == TaskStatus::Accepted
            && task.result == Some(sender.result) && task.run_ticks == 1)
        && matches!(recipient_context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(recipient.task))
        && matches!(sender_context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none())
        && matches!(waiter, Some(record) if !record.active
            && record.kind == WaiterKind::Mailbox
            && record.agent == recipient.agent
            && record.task == recipient.task)
        && kernel.run_queue()
            == [RunQueueEntry {
                task: sender.task,
                agent: sender.agent,
            }]
}
