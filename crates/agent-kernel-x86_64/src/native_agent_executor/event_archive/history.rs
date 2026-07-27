//! Terminal Event history proof for released and retained archive modes.
//!
//! This x86 execution-layer verifier reconciles the external 64-Event segment
//! with Core's terminal sequence. Durable checkpoints require a dense suffix;
//! disk-free snapshots require the complete resident history to remain intact.

use agent_kernel_core::{EventArchiveDigest, EventArchiveProposal};

use super::{NativeEventArchive, NATIVE_EVENT_ARCHIVE_CAPACITY};
use crate::{
    serial_write_line, serial_write_str, serial_write_u64, X86BootedKernel,
    X86_DURABLE_BIND_EVENT_RESERVE, X86_EVENT_ARCHIVE_WATERMARK, X86_TERMINAL_EVENT_SEQUENCE,
};

impl NativeEventArchive {
    pub(crate) fn write_terminal_history_diagnostics(&self, booted: &X86BootedKernel) {
        let events = booted.kernel().events();
        write_value("AGENT_KERNEL_EVENT_HISTORY_LIVE_LEN=", events.len() as u64);
        if let Some(first) = events.first() {
            write_value("AGENT_KERNEL_EVENT_HISTORY_LIVE_FIRST=", first.sequence);
        }
        if let Some(last) = events.last() {
            write_value("AGENT_KERNEL_EVENT_HISTORY_LIVE_LAST=", last.sequence);
        }
        write_value(
            "AGENT_KERNEL_EVENT_HISTORY_NEXT=",
            booted.kernel().next_event_sequence(),
        );
        if let Some(proposal) = self.proposal() {
            write_value(
                "AGENT_KERNEL_EVENT_HISTORY_PROPOSAL_GENERATION=",
                proposal.generation(),
            );
            write_value(
                "AGENT_KERNEL_EVENT_HISTORY_PROPOSAL_FIRST=",
                proposal.first_sequence(),
            );
            write_value(
                "AGENT_KERNEL_EVENT_HISTORY_PROPOSAL_THROUGH=",
                proposal.through_sequence(),
            );
        }
        write_value(
            "AGENT_KERNEL_EVENT_HISTORY_SOURCE_LEN=",
            self.source_live_len() as u64,
        );
    }

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

        let previous_checkpoint = match self.checkpoint() {
            Some(_) => None,
            None => kernel.event_archive_checkpoint(),
        };
        let chain_valid = match previous_checkpoint {
            Some(previous) => {
                proposal.generation() == previous.generation().checked_add(1).unwrap_or(0)
                    && proposal.first_sequence()
                        == previous.through_sequence().checked_add(1).unwrap_or(0)
                    && proposal.previous_digest() == previous.digest()
            }
            None => {
                proposal.generation() == 1
                    && proposal.first_sequence() == 1
                    && proposal.previous_digest() == EventArchiveDigest::ZERO
                    && EventArchiveProposal::from_segment(None, &segment) == Some(proposal)
            }
        };
        let common = self.source_live_len() == X86_EVENT_ARCHIVE_WATERMARK
            && self.len() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && copied == NATIVE_EVENT_ARCHIVE_CAPACITY
            && proposal.through_sequence()
                == proposal
                    .first_sequence()
                    .checked_add(NATIVE_EVENT_ARCHIVE_CAPACITY as u64 - 1)
                    .unwrap_or(0)
            && proposal.count() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && chain_valid
            && self
                .events()
                .enumerate()
                .all(|(index, event)| event.sequence == proposal.first_sequence() + index as u64);
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
                let durable_resource_events =
                    usize::from(previous_checkpoint.is_some()) * X86_DURABLE_BIND_EVENT_RESERVE;
                let expected_live_len = X86_TERMINAL_EVENT_SEQUENCE + durable_resource_events;
                let expected_last = proposal
                    .first_sequence()
                    .checked_add(expected_live_len as u64 - 1);
                let retained_head_valid = match previous_checkpoint {
                    Some(previous) => {
                        kernel.event_archive_checkpoint() == Some(previous)
                            && kernel.durable_archive_receipt().is_some_and(|receipt| {
                                receipt.generation() == previous.generation()
                                    && receipt.archive_digest() == previous.digest()
                            })
                    }
                    None => {
                        kernel.event_archive_checkpoint().is_none()
                            && kernel.durable_archive_receipt().is_none()
                    }
                };
                retained_head_valid
                    && live.len() == expected_live_len
                    && live
                        .first()
                        .is_some_and(|event| event.sequence == proposal.first_sequence())
                    && live
                        .last()
                        .is_some_and(|event| Some(event.sequence) == expected_last)
                    && Some(kernel.next_event_sequence())
                        == expected_last.and_then(|value| value.checked_add(1))
                    && live.iter().enumerate().all(|(index, event)| {
                        event.sequence == proposal.first_sequence() + index as u64
                    })
                    && self
                        .events()
                        .zip(live.iter())
                        .all(|(snapshot, resident)| snapshot == resident)
            }
        }
    }
}

fn write_value(label: &str, value: u64) {
    serial_write_str(label);
    serial_write_u64(value);
    serial_write_line("");
}
