//! Completed Worker evidence handed to the native Verifier schedule.
//!
//! This boot-layer token keeps Worker identities private while allowing the
//! Verifier adapter to prove its subject and peer remain in terminal state.

use agent_kernel_core::{
    AgentExecutionState, MessageKind, MessagePayload, MessageStatus, TaskId, TaskResult, TaskStatus,
};

use super::WorkerTask;
use crate::X86BootedKernel;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerificationSubject {
    task: TaskId,
    result: TaskResult,
}

pub(crate) struct CompletedWorkerTasks {
    first: WorkerTask,
    second: WorkerTask,
}

impl VerificationSubject {
    pub(crate) const fn new(task: TaskId, result: TaskResult) -> Self {
        Self { task, result }
    }

    pub(crate) const fn task(self) -> TaskId {
        self.task
    }

    pub(crate) const fn result(self) -> TaskResult {
        self.result
    }
}

impl CompletedWorkerTasks {
    pub(crate) const fn new(first: WorkerTask, second: WorkerTask) -> Self {
        Self { first, second }
    }

    pub(crate) const fn subject(&self) -> VerificationSubject {
        VerificationSubject::new(self.first.task, self.first.result)
    }

    pub(crate) fn both_completed(&self, booted: &X86BootedKernel) -> bool {
        task_valid(booted, self.first, 1)
            && task_valid(booted, self.second, 1)
            && self.mailbox_acknowledged(booted)
    }

    pub(crate) fn peer_completed(&self, booted: &X86BootedKernel) -> bool {
        task_valid(booted, self.second, 1) && self.mailbox_acknowledged(booted)
    }

    pub(crate) fn mailbox_acknowledged(&self, booted: &X86BootedKernel) -> bool {
        matches!(booted.kernel().messages(), [message]
            if message.sender == self.first.agent
                && message.recipient == self.second.agent
                && message.kind == MessageKind::Notify
                && message.payload == MessagePayload {
                    task: Some(self.first.task),
                    ..MessagePayload::empty()
                }
                && message.status == MessageStatus::Acknowledged)
    }
}

pub(super) fn task_valid(booted: &X86BootedKernel, worker: WorkerTask, ticks: u64) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == worker.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == worker.agent);
    matches!(task, Some(task) if task.status == TaskStatus::Completed
        && task.assignee == Some(worker.agent)
        && task.delegated_capability == Some(worker.capability)
        && task.result == Some(worker.result)
        && task.run_ticks == ticks)
        && matches!(context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none())
}
