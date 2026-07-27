//! Authorized Event snapshot or durable archive proof for the Supervisor.

use agent_kernel_core::{EventArchiveDigest, EventArchiveProposal, EventKind};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    native_agent_executor::{NativeExecutionReport, NATIVE_EVENT_ARCHIVE_CAPACITY},
    X86BootedKernel, X86_EVENT_ARCHIVE_WATERMARK,
};

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn event_history_recorded(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
    ) -> bool {
        let archive = report.event_archive();
        let Some(proposal) = archive.proposal() else {
            return false;
        };
        let Some(first) = archive.events().next().copied() else {
            return false;
        };
        let mut segment = [first; NATIVE_EVENT_ARCHIVE_CAPACITY];
        let mut copied = 0;
        for (index, event) in archive.events().copied().enumerate() {
            segment[index] = event;
            copied = index + 1;
        }
        let kernel = booted.kernel();
        let live = kernel.events();
        let previous_checkpoint = if archive.checkpoint().is_none() {
            kernel.event_archive_checkpoint()
        } else {
            None
        };
        let expected_first = previous_checkpoint
            .map(|checkpoint| checkpoint.through_sequence().checked_add(1))
            .unwrap_or(Some(1));
        let Some(expected_first) = expected_first else {
            return false;
        };
        let expected_generation = previous_checkpoint
            .map(|checkpoint| checkpoint.generation().checked_add(1))
            .unwrap_or(Some(1));
        let Some(expected_generation) = expected_generation else {
            return false;
        };
        let expected_previous_digest = previous_checkpoint
            .map(|checkpoint| checkpoint.digest())
            .unwrap_or(EventArchiveDigest::ZERO);
        let expected_through = expected_first.checked_add(NATIVE_EVENT_ARCHIVE_CAPACITY as u64 - 1);
        let terminal_sequence = expected_first.checked_add(392);
        let next_sequence = expected_first.checked_add(393);

        let common = archive.source_live_len() == X86_EVENT_ARCHIVE_WATERMARK
            && archive.len() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && copied == NATIVE_EVENT_ARCHIVE_CAPACITY
            && proposal.generation() == expected_generation
            && proposal.first_sequence() == expected_first
            && Some(proposal.through_sequence()) == expected_through
            && proposal.count() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && proposal.previous_digest() == expected_previous_digest
            && archive.actor() == Some(ADMISSION_SUPERVISOR)
            && archive.authority() == Some(self.supervisor.admission_authority)
            && archive.root() == Some(booted.report().bootstrap_resource)
            && EventArchiveProposal::from_segment(previous_checkpoint, &segment) == Some(proposal)
            && archive
                .events()
                .enumerate()
                .all(|(index, event)| event.sequence == expected_first + index as u64);
        if !common {
            return false;
        }

        let terminal_event = |event: &agent_kernel_core::Event| {
            Some(event.sequence) == terminal_sequence
                && event.kind == EventKind::TaskCompleted
                && event.agent == ADMISSION_SUPERVISOR
                && event.task == Some(self.supervisor.task)
        };
        match archive.checkpoint() {
            Some(checkpoint) => {
                checkpoint.proposal() == proposal
                    && checkpoint.actor() == ADMISSION_SUPERVISOR
                    && checkpoint.authority() == self.supervisor.admission_authority
                    && checkpoint.root() == booted.report().bootstrap_resource
                    && kernel.event_archive_checkpoint() == Some(checkpoint)
                    && live.len() == 329
                    && live.first().is_some_and(|event| {
                        event.sequence == expected_first + NATIVE_EVENT_ARCHIVE_CAPACITY as u64
                    })
                    && live.last().is_some_and(terminal_event)
                    && Some(kernel.next_event_sequence()) == next_sequence
            }
            None => {
                archive.is_retained_snapshot()
                    && kernel.event_archive_checkpoint() == previous_checkpoint
                    && kernel.durable_archive_receipt().is_some() == previous_checkpoint.is_some()
                    && live.len() == 393
                    && live
                        .first()
                        .is_some_and(|event| event.sequence == expected_first)
                    && live.last().is_some_and(terminal_event)
                    && Some(kernel.next_event_sequence()) == next_sequence
                    && archive
                        .events()
                        .zip(live.iter())
                        .all(|(snapshot, resident)| snapshot == resident)
            }
        }
    }
}
