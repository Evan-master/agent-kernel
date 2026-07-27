//! Terminal Event history proof for released and retained archive modes.
//!
//! This x86 execution-layer verifier reconciles the external 64-Event segment
//! with Core's terminal sequence. Durable checkpoints require a dense suffix;
//! disk-free snapshots require the complete resident history to remain intact.

use agent_kernel_core::{EventArchiveDigest, EventArchiveProposal};

use super::{NativeEventArchive, NATIVE_EVENT_ARCHIVE_CAPACITY};
use crate::{X86BootedKernel, X86_EVENT_ARCHIVE_WATERMARK, X86_TERMINAL_EVENT_SEQUENCE};

impl NativeEventArchive {
    pub(crate) fn proves_terminal_history(&self, booted: &X86BootedKernel) -> bool {
        let Some(proposal) = self.proposal() else {
            return false;
        };
        let Some(first) = self.events().next().copied() else {
            return false;
        };
        let mut segment = [first; NATIVE_EVENT_ARCHIVE_CAPACITY];
        let mut copied = 0;
        for (index, event) in self.events().copied().enumerate() {
            segment[index] = event;
            copied = index + 1;
        }
        let kernel = booted.kernel();
        let live = kernel.events();

        let common = self.source_live_len() == X86_EVENT_ARCHIVE_WATERMARK
            && self.len() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && copied == NATIVE_EVENT_ARCHIVE_CAPACITY
            && proposal.generation() == 1
            && proposal.first_sequence() == 1
            && proposal.through_sequence() == NATIVE_EVENT_ARCHIVE_CAPACITY as u64
            && proposal.count() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && proposal.previous_digest() == EventArchiveDigest::ZERO
            && EventArchiveProposal::from_segment(None, &segment) == Some(proposal);
        if !common {
            return false;
        }

        match self.checkpoint() {
            Some(checkpoint) => {
                checkpoint.proposal() == proposal
                    && kernel.event_archive_checkpoint() == Some(checkpoint)
                    && live.len() + self.len() == X86_TERMINAL_EVENT_SEQUENCE
                    && live.first().is_some_and(|event| event.sequence == 65)
                    && live
                        .last()
                        .is_some_and(|event| event.sequence == X86_TERMINAL_EVENT_SEQUENCE as u64)
                    && kernel.next_event_sequence() == X86_TERMINAL_EVENT_SEQUENCE as u64 + 1
                    && self
                        .events()
                        .chain(live.iter())
                        .enumerate()
                        .all(|(index, event)| event.sequence == index as u64 + 1)
            }
            None => {
                kernel.event_archive_checkpoint().is_none()
                    && kernel.durable_archive_receipt().is_none()
                    && live.len() == X86_TERMINAL_EVENT_SEQUENCE
                    && live.first().is_some_and(|event| event.sequence == 1)
                    && live
                        .last()
                        .is_some_and(|event| event.sequence == X86_TERMINAL_EVENT_SEQUENCE as u64)
                    && kernel.next_event_sequence() == X86_TERMINAL_EVENT_SEQUENCE as u64 + 1
                    && live
                        .iter()
                        .enumerate()
                        .all(|(index, event)| event.sequence == index as u64 + 1)
                    && self
                        .events()
                        .zip(live.iter())
                        .all(|(snapshot, resident)| snapshot == resident)
            }
        }
    }
}
