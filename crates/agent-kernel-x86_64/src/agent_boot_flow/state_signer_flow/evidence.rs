//! Exact semantic, transcript, and durable receipt proof for StateSigner.

use agent_kernel_core::{AgentExecutionState, EventKind, IntentStatus, TaskResult, TaskStatus};
use agent_kernel_x86_64::{agent_call::AgentCallOperation, ata::AtaDurableHead};

use super::PreparedStateSigner;
use crate::{
    boot_config::StateSignerBootProfile,
    native_agent_executor::{NativeEventArchive, NativeExecutionReport},
    NativeDurableSession, X86BootedKernel,
};

const OPERATIONS: [AgentCallOperation; 6] = [
    AgentCallOperation::DescribeContext,
    AgentCallOperation::PrepareDurableArchive,
    AgentCallOperation::SignDurableArchive,
    AgentCallOperation::CommitDurableArchiveFromMemory,
    AgentCallOperation::SubmitTaskResult,
    AgentCallOperation::CompleteTask,
];

pub(super) fn completed(
    booted: &X86BootedKernel,
    report: &NativeExecutionReport,
    prepared: PreparedStateSigner,
    profile: StateSignerBootProfile,
    retained_snapshot: &NativeEventArchive,
    session: &NativeDurableSession<'_>,
) -> bool {
    let Some(completed) = report.completed(profile.agent()) else {
        return false;
    };
    let Some(committed) = report.event_archive().checkpoint() else {
        return false;
    };
    let Some(proposal) = retained_snapshot.proposal() else {
        return false;
    };
    let kernel = booted.kernel();
    let Some(context) = prepared.context(profile) else {
        return false;
    };
    let task = kernel
        .tasks()
        .iter()
        .find(|record| record.id == prepared.task);
    let intent = kernel
        .intents()
        .iter()
        .find(|record| record.id == prepared.intent);
    let execution = kernel
        .execution_contexts()
        .iter()
        .find(|record| record.agent == profile.agent());
    let receipt = kernel.durable_archive_receipt();
    let expected_result = TaskResult {
        code: 0x0a17,
        value: profile.call_data_generation(),
    };

    matches!(task, Some(task)
        if task.status == TaskStatus::Completed
            && task.result == Some(expected_result)
            && task.assignee == Some(profile.agent()))
        && matches!(intent, Some(intent) if intent.status == IntentStatus::Bound)
        && matches!(execution, Some(execution)
            if execution.state == AgentExecutionState::Idle && execution.task.is_none())
        && completed.context() == context
        && completed.nonce() == profile.nonce()
        && completed.call_count() == 6
        && completed.address_space_switch_count() == 12
        && completed.operations() == OPERATIONS
        && completed.return_offsets() == profile.return_offsets()
        && completed.physical_quantum_generation() == 1
        && completed.restart_generation() == 0
        && completed.lazy_data_byte() == 0
        && completed.runtime_page_generation() == 0
        && !completed.runtime_page_released()
        && completed.runtime_page_observation().is_none()
        && completed.runtime_region_generation() == 0
        && !completed.runtime_regions_released()
        && completed.runtime_region_observations().is_empty()
        && completed.reclamation_log().is_empty()
        && report.event_archive().is_released()
        && report.event_archive().len() == retained_snapshot.len()
        && report.event_archive().proposal() == Some(proposal)
        && report
            .event_archive()
            .events()
            .zip(retained_snapshot.events())
            .all(|(committed, retained)| committed == retained)
        && committed.proposal() == proposal
        && committed.actor() == profile.agent()
        && committed.authority() == profile.archive_authority()
        && kernel.event_archive_checkpoint() == Some(committed)
        && matches!(receipt, Some(receipt)
            if receipt.generation() == 1
                && receipt.storage() == session.config().storage()
                && receipt.archive_digest() == proposal.digest()
                && receipt.flush_epoch() > 0)
        && session.backend().head() == Some(AtaDurableHead::Recovered(1))
        && kernel
            .events()
            .first()
            .is_some_and(|event| event.sequence == 65)
        && kernel.events().iter().any(|event| {
            event.kind == EventKind::TaskCompleted
                && event.agent == profile.agent()
                && event.task == Some(prepared.task)
        })
}
