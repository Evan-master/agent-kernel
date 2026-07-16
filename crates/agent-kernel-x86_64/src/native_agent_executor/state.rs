//! Public kernel-state predicates used by the native execution loop.
//!
//! These validators read only facade-exposed records. They bind physical call
//! ownership to task and execution-context state without reaching into core
//! stores or encoding role-specific Capsule sequences.

use agent_kernel_core::{AgentExecutionState, RunQueueEntry, TaskStatus};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::X86BootedKernel;

pub(super) fn running(booted: &X86BootedKernel, expected: AgentCallContext) -> bool {
    running_progress(booted, expected).is_some()
}

pub(super) fn running_progress(
    booted: &X86BootedKernel,
    expected: AgentCallContext,
) -> Option<(u64, u64)> {
    let kernel = booted.kernel();
    let task = kernel
        .tasks()
        .iter()
        .find(|task| task.id == expected.task());
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == expected.agent());
    let task = task.filter(|task| {
        task.status == TaskStatus::Running
            && task.assignee == Some(expected.agent())
            && task.delegated_capability == Some(expected.capability())
    })?;
    matches!(context, Some(context)
            if context.state == AgentExecutionState::Running
                && context.task == Some(expected.task()))
    .then_some((task.run_ticks, task.quantum_remaining))
}

pub(super) fn queued(booted: &X86BootedKernel, expected: AgentCallContext) -> bool {
    let kernel = booted.kernel();
    let task = kernel
        .tasks()
        .iter()
        .find(|task| task.id == expected.task());
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == expected.agent());
    matches!(task, Some(task)
        if task.status == TaskStatus::Accepted
            && task.assignee == Some(expected.agent())
            && task.delegated_capability == Some(expected.capability()))
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Idle && context.task.is_none())
        && kernel.run_queue().contains(&RunQueueEntry {
            task: expected.task(),
            agent: expected.agent(),
        })
}

pub(super) fn waiting(booted: &X86BootedKernel, expected: AgentCallContext) -> bool {
    let kernel = booted.kernel();
    let task = kernel
        .tasks()
        .iter()
        .find(|task| task.id == expected.task());
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == expected.agent());
    matches!(task, Some(task)
        if task.status == TaskStatus::Waiting
            && task.assignee == Some(expected.agent())
            && task.delegated_capability == Some(expected.capability()))
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Waiting
                && context.task == Some(expected.task()))
}

pub(super) fn completed(booted: &X86BootedKernel, expected: AgentCallContext) -> bool {
    let kernel = booted.kernel();
    let task = kernel
        .tasks()
        .iter()
        .find(|task| task.id == expected.task());
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == expected.agent());
    matches!(task, Some(task)
        if task.status == TaskStatus::Completed
            && task.assignee == Some(expected.agent())
            && task.delegated_capability == Some(expected.capability()))
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Idle && context.task.is_none())
}

pub(super) fn faulted(booted: &X86BootedKernel, expected: AgentCallContext) -> bool {
    let kernel = booted.kernel();
    let task = kernel
        .tasks()
        .iter()
        .find(|task| task.id == expected.task());
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == expected.agent());
    matches!(task, Some(task)
        if task.status == TaskStatus::Faulted
            && task.assignee == Some(expected.agent())
            && task.delegated_capability == Some(expected.capability())
            && task.last_fault.is_some())
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Faulted
                && context.task == Some(expected.task()))
}

pub(super) fn verified(booted: &X86BootedKernel, task: agent_kernel_core::TaskId) -> bool {
    matches!(
        booted.kernel().tasks().iter().find(|record| record.id == task),
        Some(record) if record.status == TaskStatus::Verified && record.result.is_some()
    )
}
