//! Architecture-owned handoff buffer for committed Event archives.
//!
//! This x86 execution-layer store retains one complete bounded Event segment
//! outside Core, together with its checkpoint and source occupancy. It enables
//! exact replay evidence after Core releases the corresponding live slots.

use agent_kernel_core::{Event, EventArchiveCheckpoint, EventArchiveDigest, EventArchiveProposal};

use super::NativeExecutionReport;
use crate::{X86BootedKernel, X86_EVENT_CAPACITY};

pub(crate) const NATIVE_EVENT_ARCHIVE_CAPACITY: usize = 64;
const TERMINAL_EVENT_SEQUENCE: u64 = 409;

pub(crate) struct NativeEventArchive {
    events: [Option<Event>; NATIVE_EVENT_ARCHIVE_CAPACITY],
    len: usize,
    source_live_len: usize,
    checkpoint: Option<EventArchiveCheckpoint>,
}

impl NativeEventArchive {
    pub(super) const fn new() -> Self {
        Self {
            events: [None; NATIVE_EVENT_ARCHIVE_CAPACITY],
            len: 0,
            source_live_len: 0,
            checkpoint: None,
        }
    }

    pub(super) const fn can_record(&self, count: usize) -> bool {
        self.checkpoint.is_none()
            && count > 0
            && count <= NATIVE_EVENT_ARCHIVE_CAPACITY.saturating_sub(self.len)
    }

    pub(super) fn record(
        &mut self,
        source_live_len: usize,
        events: &[Option<Event>],
        checkpoint: EventArchiveCheckpoint,
    ) -> Option<()> {
        if !self.can_record(events.len())
            || events.iter().any(Option::is_none)
            || checkpoint.count() != events.len()
            || checkpoint.first_sequence() != events.first()?.as_ref()?.sequence
            || checkpoint.through_sequence() != events.last()?.as_ref()?.sequence
        {
            return None;
        }
        for (index, event) in events.iter().copied().enumerate() {
            self.events[index] = event;
        }
        self.len = events.len();
        self.source_live_len = source_live_len;
        self.checkpoint = Some(checkpoint);
        Some(())
    }

    pub(crate) fn events(&self) -> impl Iterator<Item = &Event> {
        self.events[..self.len].iter().flatten()
    }

    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    pub(crate) const fn source_live_len(&self) -> usize {
        self.source_live_len
    }

    pub(crate) const fn checkpoint(&self) -> Option<EventArchiveCheckpoint> {
        self.checkpoint
    }

    pub(crate) fn proves_terminal_replay(&self, booted: &X86BootedKernel) -> bool {
        let Some(checkpoint) = self.checkpoint else {
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

        self.source_live_len == X86_EVENT_CAPACITY
            && self.len == NATIVE_EVENT_ARCHIVE_CAPACITY
            && copied == NATIVE_EVENT_ARCHIVE_CAPACITY
            && checkpoint.generation() == 1
            && checkpoint.first_sequence() == 1
            && checkpoint.through_sequence() == NATIVE_EVENT_ARCHIVE_CAPACITY as u64
            && checkpoint.count() == NATIVE_EVENT_ARCHIVE_CAPACITY
            && checkpoint.previous_digest() == EventArchiveDigest::ZERO
            && kernel.event_archive_checkpoint() == Some(checkpoint)
            && EventArchiveProposal::from_segment(None, &segment) == Some(checkpoint.proposal())
            && live.len() + self.len == TERMINAL_EVENT_SEQUENCE as usize
            && live.first().is_some_and(|event| event.sequence == 65)
            && live
                .last()
                .is_some_and(|event| event.sequence == TERMINAL_EVENT_SEQUENCE)
            && kernel.next_event_sequence() == TERMINAL_EVENT_SEQUENCE + 1
            && self
                .events()
                .chain(live.iter())
                .enumerate()
                .all(|(index, event)| event.sequence == index as u64 + 1)
    }
}

impl NativeExecutionReport {
    pub(super) const fn can_record_event_archive(&self, count: usize) -> bool {
        self.event_archive.can_record(count)
    }

    pub(super) fn record_event_archive(
        &mut self,
        source_live_len: usize,
        events: &[Option<Event>],
        checkpoint: EventArchiveCheckpoint,
    ) -> Option<()> {
        self.event_archive
            .record(source_live_len, events, checkpoint)
    }

    pub(crate) const fn event_archive(&self) -> &NativeEventArchive {
        &self.event_archive
    }

    pub(crate) fn into_event_archive(self) -> NativeEventArchive {
        self.event_archive
    }
}
