//! Semantic mailbox wait and sender dispatch for the x86 Worker schedule.
//!
//! This boot-layer child binds a captured ReceiveMessage frame to one core
//! mailbox waiter. It validates scheduler ownership before and after the public
//! wait and dispatch syscalls; the CPU token retains the ring-3 frame.

use agent_kernel_core::{
    AgentExecutionState, EventKind, MessageReceiveOutcome, RunQueueEntry, TaskStatus, WaiterId,
    WaiterKind,
};

use super::{WorkerTask, TASK_QUANTUM};
use crate::{
    agent_cpu::{RequestedMessageReceiveCpu, WaitingMessageReceiveCpu},
    X86BootedKernel,
};

pub(super) fn wait(
    booted: &mut X86BootedKernel,
    receiver: WorkerTask,
    sender: WorkerTask,
    cpu: RequestedMessageReceiveCpu,
) -> Option<(WaiterId, WaitingMessageReceiveCpu)> {
    if cpu.call_count() != 2
        || cpu.address_space_switch_count() != 4
        || receiver.call_context() != Some(cpu.context())
        || !receiver_ready_to_wait(booted, receiver, sender)
    {
        return None;
    }
    let MessageReceiveOutcome::Waiting(waiter) = booted
        .kernel_mut()
        .sys_receive_or_wait_message(receiver.agent, receiver.capability, receiver.task)
        .ok()?
    else {
        return None;
    };
    let event = booted.kernel().events().last()?;
    if event.kind != EventKind::MessageWaitStarted
        || event.agent != receiver.agent
        || event.capability != Some(receiver.capability)
        || event.task != Some(receiver.task)
        || event.waiter != Some(waiter)
        || event.message.is_some()
        || !receiver_waiting_state_valid(booted, receiver, sender, waiter)
    {
        return None;
    }
    Some((waiter, cpu.wait(waiter)?))
}

pub(super) fn dispatch_sender(
    booted: &mut X86BootedKernel,
    sender: WorkerTask,
    receiver: WorkerTask,
    waiter: WaiterId,
) -> Option<RunQueueEntry> {
    if !receiver_waiting_state_valid(booted, receiver, sender, waiter) {
        return None;
    }
    let dispatched = booted
        .kernel_mut()
        .sys_dispatch_next_ready_with_quantum(TASK_QUANTUM)
        .ok()?;
    if dispatched
        != (RunQueueEntry {
            task: sender.task,
            agent: sender.agent,
        })
        || !sender_running_receiver_waiting_state_valid(booted, sender, receiver, waiter)
    {
        return None;
    }
    Some(dispatched)
}

pub(super) fn receiver_waiting_state_valid(
    booted: &X86BootedKernel,
    receiver: WorkerTask,
    sender: WorkerTask,
    waiter: WaiterId,
) -> bool {
    let kernel = booted.kernel();
    let receiver_task = kernel.tasks().iter().find(|task| task.id == receiver.task);
    let sender_task = kernel.tasks().iter().find(|task| task.id == sender.task);
    let receiver_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == receiver.agent);
    let sender_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == sender.agent);
    let waiter_record = kernel.waiters().iter().find(|record| record.id == waiter);
    matches!(receiver_task, Some(task) if task.status == TaskStatus::Waiting
        && task.assignee == Some(receiver.agent)
        && task.delegated_capability == Some(receiver.capability)
        && task.result.is_none()
        && task.run_ticks == 1)
        && matches!(sender_task, Some(task) if task.status == TaskStatus::Accepted
            && task.assignee == Some(sender.agent)
            && task.delegated_capability == Some(sender.capability)
            && task.result.is_none()
            && task.run_ticks == 1)
        && matches!(receiver_context, Some(context) if context.state == AgentExecutionState::Waiting
            && context.task == Some(receiver.task))
        && matches!(sender_context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none())
        && matches!(waiter_record, Some(record) if record.active
            && record.kind == WaiterKind::Mailbox
            && record.agent == receiver.agent
            && record.task == receiver.task
            && record.resource == receiver_resource(booted, receiver))
        && kernel.messages().is_empty()
        && kernel.run_queue()
            == [RunQueueEntry {
                task: sender.task,
                agent: sender.agent,
            }]
}

pub(super) fn sender_running_receiver_waiting_state_valid(
    booted: &X86BootedKernel,
    sender: WorkerTask,
    receiver: WorkerTask,
    waiter: WaiterId,
) -> bool {
    if !receiver_waiting_without_queue_valid(booted, receiver, waiter) {
        return false;
    }
    let kernel = booted.kernel();
    let sender_task = kernel.tasks().iter().find(|task| task.id == sender.task);
    let sender_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == sender.agent);
    let event = kernel.events().last();
    matches!(sender_task, Some(task) if task.status == TaskStatus::Running
        && task.assignee == Some(sender.agent)
        && task.delegated_capability == Some(sender.capability)
        && task.result.is_none()
        && task.run_ticks == 1
        && task.quantum_remaining == TASK_QUANTUM)
        && matches!(sender_context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(sender.task)
            && context.run_ticks == 1
            && context.quantum_remaining == TASK_QUANTUM)
        && matches!(event, Some(event) if event.kind == EventKind::TaskDispatched
            && event.task == Some(sender.task))
        && kernel.run_queue().is_empty()
}

fn receiver_ready_to_wait(
    booted: &X86BootedKernel,
    receiver: WorkerTask,
    sender: WorkerTask,
) -> bool {
    let kernel = booted.kernel();
    let receiver_task = kernel.tasks().iter().find(|task| task.id == receiver.task);
    let receiver_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == receiver.agent);
    matches!(receiver_task, Some(task) if task.status == TaskStatus::Running
        && task.result.is_none() && task.run_ticks == 1
        && task.quantum_remaining == TASK_QUANTUM)
        && matches!(receiver_context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(receiver.task))
        && kernel.waiters().is_empty()
        && kernel.messages().is_empty()
        && kernel.run_queue()
            == [RunQueueEntry {
                task: sender.task,
                agent: sender.agent,
            }]
}

fn receiver_waiting_without_queue_valid(
    booted: &X86BootedKernel,
    receiver: WorkerTask,
    waiter: WaiterId,
) -> bool {
    let kernel = booted.kernel();
    let receiver_task = kernel.tasks().iter().find(|task| task.id == receiver.task);
    let receiver_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == receiver.agent);
    let waiter_record = kernel.waiters().iter().find(|record| record.id == waiter);
    matches!(receiver_task, Some(task) if task.status == TaskStatus::Waiting
        && task.assignee == Some(receiver.agent)
        && task.result.is_none()
        && task.run_ticks == 1)
        && matches!(receiver_context, Some(context) if context.state == AgentExecutionState::Waiting
            && context.task == Some(receiver.task))
        && matches!(waiter_record, Some(record) if record.active
            && record.kind == WaiterKind::Mailbox
            && record.agent == receiver.agent
            && record.task == receiver.task)
}

fn receiver_resource(
    booted: &X86BootedKernel,
    receiver: WorkerTask,
) -> agent_kernel_core::ResourceId {
    booted
        .kernel()
        .tasks()
        .iter()
        .find(|task| task.id == receiver.task)
        .map(|task| task.resource)
        .unwrap_or(agent_kernel_core::ResourceId::new(0))
}
