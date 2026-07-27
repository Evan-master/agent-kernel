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

        let common = archive.source_live_len() == X86_EVENT_ARCHIVE_WATERMARK
            && archive.len() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && copied == NATIVE_EVENT_ARCHIVE_CAPACITY
            && proposal.generation() == 1
            && proposal.first_sequence() == 1
            && proposal.through_sequence() == NATIVE_EVENT_ARCHIVE_CAPACITY as u64
            && proposal.count() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && proposal.previous_digest() == EventArchiveDigest::ZERO
            && archive.actor() == Some(ADMISSION_SUPERVISOR)
            && archive.authority() == Some(self.supervisor.admission_authority)
            && archive.root() == Some(booted.report().bootstrap_resource)
            && EventArchiveProposal::from_segment(None, &segment) == Some(proposal)
            && archive
                .events()
                .enumerate()
                .all(|(index, event)| event.sequence == index as u64 + 1);
        if !common {
            return false;
        }

        let terminal_event = |event: &agent_kernel_core::Event| {
            event.sequence == 393
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
                    && live.first().is_some_and(|event| event.sequence == 65)
                    && live.last().is_some_and(terminal_event)
                    && kernel.next_event_sequence() == 394
            }
            None => {
                archive.is_retained_snapshot()
                    && kernel.event_archive_checkpoint().is_none()
                    && kernel.durable_archive_receipt().is_none()
                    && live.len() == 393
                    && live.first().is_some_and(|event| event.sequence == 1)
                    && live.last().is_some_and(terminal_event)
                    && kernel.next_event_sequence() == 394
                    && archive
                        .events()
                        .zip(live.iter())
                        .all(|(snapshot, resident)| snapshot == resident)
            }
        }
    }
}
