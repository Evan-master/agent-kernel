//! Verifier queue and terminal predicates for the generic runtime loop.
//!
//! This semantic adapter retains trusted Verifier metadata while physical call
//! sequencing is owned by the role-independent executor.

use agent_kernel_core::{AgentExecutionState, EventKind, RunQueueEntry, TaskStatus};

use super::VerifierTask;
use crate::{timer_task_flow::CompletedWorkerTasks, X86BootedKernel};

pub(super) fn queue(
    booted: &mut X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
    predecessor: Option<RunQueueEntry>,
) -> Option<()> {
    let existing_queue_valid = match predecessor {
        Some(entry) => booted.kernel().run_queue() == [entry],
        None => booted.kernel().run_queue().is_empty(),
    };
    if !workers.both_completed(booted) || !existing_queue_valid {
        return None;
    }
    let event = booted
        .kernel_mut()
        .sys_enqueue_task(verifier.agent, verifier.task)
        .ok()?;
    (event.kind == EventKind::TaskQueued
        && event.agent == verifier.agent
        && event.task == Some(verifier.task)
        && match predecessor {
            Some(entry) => {
                booted.kernel().run_queue()
                    == [
                        entry,
                        RunQueueEntry {
                            task: verifier.task,
                            agent: verifier.agent,
                        },
                    ]
            }
            None => {
                booted.kernel().run_queue()
                    == [RunQueueEntry {
                        task: verifier.task,
                        agent: verifier.agent,
                    }]
            }
        })
    .then_some(())
}

pub(super) fn completed(
    booted: &X86BootedKernel,
    verifier: VerifierTask,
    workers: &CompletedWorkerTasks,
) -> bool {
    let kernel = booted.kernel();
    let task = kernel.tasks().iter().find(|task| task.id == verifier.task);
    let target = kernel
        .tasks()
        .iter()
        .find(|task| task.id == verifier.subject.task());
    let context = kernel
        .execution_contexts()
        .iter()
        .find(|context| context.agent == verifier.agent);
    matches!(task, Some(task)
        if task.status == TaskStatus::Completed
            && task.run_ticks == 1
            && task.result.is_none())
        && matches!(target, Some(task)
            if task.status == TaskStatus::Verified
                && task.result == Some(verifier.subject.result()))
        && matches!(context, Some(context)
            if context.state == AgentExecutionState::Idle && context.task.is_none())
        && workers.peer_completed(booted)
        && kernel.run_queue().is_empty()
}
