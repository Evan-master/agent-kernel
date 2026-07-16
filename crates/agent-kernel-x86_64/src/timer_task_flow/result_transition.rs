//! Returning task-result transition for the x86 boot adapter.
//!
//! This architecture-layer module binds captured CPU evidence to the public
//! task-result syscall, validates its event and unchanged scheduler state, and
//! only then creates a reply-capable CPU token. It performs no ABI decoding.

use agent_kernel_core::{AgentExecutionState, EventKind, RunQueueEntry, TaskStatus};

use super::{WorkerTask, TASK_QUANTUM};
use crate::{
    agent_cpu::{
        AcknowledgedReceiverResultCpu, AcknowledgedSenderResultCpu, RequestedReceiverResultCpu,
        RequestedSenderResultCpu,
    },
    X86BootedKernel,
};

pub(super) trait ResultEvidence: Sized {
    type Acknowledged;
    const EXPECTED_CALLS: u8;

    fn call_count(&self) -> u8;
    fn address_space_switch_count(&self) -> u8;
    fn context(&self) -> agent_kernel_x86_64::agent_call::AgentCallContext;
    fn result(&self) -> agent_kernel_core::TaskResult;
    fn acknowledge(self) -> Option<Self::Acknowledged>;
}

pub(super) fn submit<E: ResultEvidence>(
    booted: &mut X86BootedKernel,
    running: WorkerTask,
    queued: Option<WorkerTask>,
    completed: Option<WorkerTask>,
    cpu: E,
) -> Option<E::Acknowledged> {
    if cpu.call_count() != E::EXPECTED_CALLS
        || cpu.address_space_switch_count() != E::EXPECTED_CALLS * 2
        || running.call_context() != Some(cpu.context())
        || cpu.result() != running.result
    {
        return None;
    }

    let event = booted
        .kernel_mut()
        .sys_submit_task_result(
            running.agent,
            running.capability,
            running.task,
            running.result,
        )
        .ok()?;
    if event.kind != EventKind::TaskResultSubmitted
        || event.agent != running.agent
        || event.task != Some(running.task)
        || event.capability != Some(running.capability)
        || event.task_result != Some(running.result)
        || !running_result_valid(booted, running)
        || !queue_valid(booted, queued)
        || !completed.is_none_or(|worker| completed_task_valid(booted, worker))
    {
        return None;
    }
    cpu.acknowledge()
}

macro_rules! impl_result_evidence {
    ($requested:ty, $acknowledged:ty, $calls:expr) => {
        impl ResultEvidence for $requested {
            type Acknowledged = $acknowledged;
            const EXPECTED_CALLS: u8 = $calls;

            fn call_count(&self) -> u8 {
                self.call_count()
            }

            fn address_space_switch_count(&self) -> u8 {
                self.address_space_switch_count()
            }

            fn context(&self) -> agent_kernel_x86_64::agent_call::AgentCallContext {
                self.context()
            }

            fn result(&self) -> agent_kernel_core::TaskResult {
                self.result()
            }

            fn acknowledge(self) -> Option<Self::Acknowledged> {
                self.acknowledge()
            }
        }
    };
}

impl_result_evidence!(RequestedSenderResultCpu, AcknowledgedSenderResultCpu, 2);
impl_result_evidence!(RequestedReceiverResultCpu, AcknowledgedReceiverResultCpu, 4);

fn running_result_valid(booted: &X86BootedKernel, worker: WorkerTask) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == worker.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == worker.agent);
    matches!(task, Some(task) if task.status == TaskStatus::Running
        && task.assignee == Some(worker.agent)
        && task.delegated_capability == Some(worker.capability)
        && task.result == Some(worker.result)
        && task.run_ticks == 1
        && task.quantum_remaining == TASK_QUANTUM)
        && matches!(context, Some(context) if context.state == AgentExecutionState::Running
            && context.task == Some(worker.task)
            && context.run_ticks == 1
            && context.quantum_remaining == TASK_QUANTUM)
}

fn queue_valid(booted: &X86BootedKernel, queued: Option<WorkerTask>) -> bool {
    match queued {
        Some(worker) => {
            booted.kernel().run_queue()
                == [RunQueueEntry {
                    task: worker.task,
                    agent: worker.agent,
                }]
                && idle_task_valid(booted, worker)
        }
        None => booted.kernel().run_queue().is_empty(),
    }
}

fn idle_task_valid(booted: &X86BootedKernel, worker: WorkerTask) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == worker.task);
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == worker.agent);
    matches!(task, Some(task) if task.status == TaskStatus::Accepted
        && task.assignee == Some(worker.agent)
        && task.delegated_capability == Some(worker.capability)
        && task.result.is_none()
        && task.run_ticks == 1)
        && matches!(context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none()
            && context.run_ticks == 0
            && context.quantum_remaining == 0)
}

fn completed_task_valid(booted: &X86BootedKernel, worker: WorkerTask) -> bool {
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
        && task.run_ticks == 1)
        && matches!(context, Some(context) if context.state == AgentExecutionState::Idle
            && context.task.is_none()
            && context.run_ticks == 0
            && context.quantum_remaining == 0)
}
