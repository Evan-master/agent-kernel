//! Cooperative Yield transition bound to an owned x86 Agent call frame.
//!
//! This boot-semantic child validates Worker A's request, performs the public
//! FIFO yield, parks the acknowledged frame, and commits Worker B only after
//! its mailbox-waiting context passes the native readiness check.

use agent_kernel_core::{AgentExecutionState, EventKind, RunQueueEntry, TaskStatus};

use super::{WorkerTask, TASK_QUANTUM};
use crate::{
    agent_cpu::RequestedSenderYieldCpu,
    native_agent_runtime::{NativeAgentContextKind, NativeAgentRuntime},
    X86BootedKernel,
};

pub(super) fn yield_and_dispatch(
    booted: &mut X86BootedKernel,
    sender: WorkerTask,
    receiver: WorkerTask,
    cpu: RequestedSenderYieldCpu,
    runtime: &mut NativeAgentRuntime,
) -> Option<RunQueueEntry> {
    if cpu.call_count() != 4
        || cpu.address_space_switch_count() != 8
        || sender.call_context() != Some(cpu.context())
        || cpu.message().raw() != 1
        || !state_valid(booted, sender, receiver, true, false)
    {
        return None;
    }

    let event = booted
        .kernel_mut()
        .sys_yield_task(sender.agent, sender.task)
        .ok()?;
    if event.kind != EventKind::TaskYielded
        || event.agent != sender.agent
        || event.task != Some(sender.task)
        || !state_valid(booted, sender, receiver, false, false)
    {
        return None;
    }

    let yielded = cpu.acknowledge()?;
    if runtime.park_yielded(yielded).is_some() {
        return None;
    }
    let expected = RunQueueEntry {
        task: receiver.task,
        agent: receiver.agent,
    };
    let dispatched = runtime.commit_ready_dispatch(
        booted,
        TASK_QUANTUM,
        expected,
        NativeAgentContextKind::WaitingMailbox,
    )?;
    state_valid(booted, sender, receiver, false, true).then_some(dispatched)
}

fn state_valid(
    booted: &X86BootedKernel,
    sender: WorkerTask,
    receiver: WorkerTask,
    sender_running: bool,
    receiver_running: bool,
) -> bool {
    let kernel = booted.kernel();
    let sender_task = kernel.tasks().iter().find(|task| task.id == sender.task);
    let receiver_task = kernel.tasks().iter().find(|task| task.id == receiver.task);
    let sender_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == sender.agent);
    let receiver_context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == receiver.agent);

    let expected_queue = match (sender_running, receiver_running) {
        (true, false) => &[RunQueueEntry {
            task: receiver.task,
            agent: receiver.agent,
        }][..],
        (false, false) => &[
            RunQueueEntry {
                task: receiver.task,
                agent: receiver.agent,
            },
            RunQueueEntry {
                task: sender.task,
                agent: sender.agent,
            },
        ][..],
        (false, true) => &[RunQueueEntry {
            task: sender.task,
            agent: sender.agent,
        }][..],
        (true, true) => return false,
    };

    matches!(sender_task, Some(task)
        if task.status == if sender_running { TaskStatus::Running } else { TaskStatus::Accepted }
            && task.assignee == Some(sender.agent)
            && task.delegated_capability == Some(sender.capability)
            && task.result == Some(sender.result)
            && task.run_ticks == 1)
        && matches!(receiver_task, Some(task)
            if task.status == if receiver_running { TaskStatus::Running } else { TaskStatus::Accepted }
                && task.assignee == Some(receiver.agent)
                && task.delegated_capability == Some(receiver.capability)
                && task.result.is_none()
                && task.run_ticks == 1)
        && matches!(sender_context, Some(context)
            if context.state == if sender_running {
                AgentExecutionState::Running
            } else {
                AgentExecutionState::Idle
            }
                && context.task == sender_running.then_some(sender.task))
        && matches!(receiver_context, Some(context)
            if context.state == if receiver_running {
                AgentExecutionState::Running
            } else {
                AgentExecutionState::Idle
            }
                && context.task == receiver_running.then_some(receiver.task))
        && kernel.run_queue() == expected_queue
}
