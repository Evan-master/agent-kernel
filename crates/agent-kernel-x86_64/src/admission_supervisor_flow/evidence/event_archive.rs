//! Full-log archive checkpoint proof for the resident Supervisor.

use agent_kernel_core::{EventArchiveDigest, EventArchiveProposal, EventKind};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    native_agent_executor::{NativeExecutionReport, NATIVE_EVENT_ARCHIVE_CAPACITY},
    X86BootedKernel, X86_EVENT_CAPACITY,
};

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn event_archive_committed(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
    ) -> bool {
        let archive = report.event_archive();
        let Some(checkpoint) = archive.checkpoint() else {
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

        archive.source_live_len() == X86_EVENT_CAPACITY
            && archive.len() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && copied == NATIVE_EVENT_ARCHIVE_CAPACITY
            && checkpoint.generation() == 1
            && checkpoint.first_sequence() == 1
            && checkpoint.through_sequence() == NATIVE_EVENT_ARCHIVE_CAPACITY as u64
            && checkpoint.count() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && checkpoint.previous_digest() == EventArchiveDigest::ZERO
            && checkpoint.actor() == ADMISSION_SUPERVISOR
            && checkpoint.authority() == self.supervisor.admission_authority
            && checkpoint.root() == booted.report().bootstrap_resource
            && kernel.event_archive_checkpoint() == Some(checkpoint)
            && EventArchiveProposal::from_segment(None, &segment) == Some(checkpoint.proposal())
            && archive
                .events()
                .enumerate()
                .all(|(index, event)| event.sequence == index as u64 + 1)
            && live.len() == 322
            && live.first().is_some_and(|event| event.sequence == 65)
            && live.last().is_some_and(|event| {
                event.sequence == 386
                    && event.kind == EventKind::TaskCompleted
                    && event.agent == ADMISSION_SUPERVISOR
                    && event.task == Some(self.supervisor.task)
            })
            && kernel.next_event_sequence() == 387
    }
}
