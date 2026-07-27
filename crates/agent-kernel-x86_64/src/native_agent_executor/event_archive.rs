//! Architecture-owned handoff buffer for Event archive evidence.
//!
//! This x86 execution-layer store retains one complete bounded Event segment
//! outside Core, together with its proposal, optional durable checkpoint, and
//! source occupancy. Disk-free boots retain Core Events; durable boots may
//! release the committed prefix after receipt verification.

mod history;

use agent_kernel_core::{
    AgentId, CapabilityId, Event, EventArchiveCheckpoint, EventArchiveProposal,
    EventArchiveSnapshot, ResourceId,
};

use super::NativeExecutionReport;

pub(crate) const NATIVE_EVENT_ARCHIVE_CAPACITY: usize = 64;

pub(crate) struct NativeEventArchive {
    events: [Option<Event>; NATIVE_EVENT_ARCHIVE_CAPACITY],
    len: usize,
    source_live_len: usize,
    proposal: Option<EventArchiveProposal>,
    checkpoint: Option<EventArchiveCheckpoint>,
    actor: Option<AgentId>,
    authority: Option<CapabilityId>,
    root: Option<ResourceId>,
}

impl NativeEventArchive {
    pub(super) const fn new() -> Self {
        Self {
            events: [None; NATIVE_EVENT_ARCHIVE_CAPACITY],
            len: 0,
            source_live_len: 0,
            proposal: None,
            checkpoint: None,
            actor: None,
            authority: None,
            root: None,
        }
    }

    pub(super) const fn can_record(&self, count: usize) -> bool {
        self.proposal.is_none()
            && count > 0
            && count <= NATIVE_EVENT_ARCHIVE_CAPACITY.saturating_sub(self.len)
    }

    pub(super) fn record_checkpoint(
        &mut self,
        source_live_len: usize,
        events: &[Option<Event>],
        checkpoint: EventArchiveCheckpoint,
    ) -> Option<()> {
        self.record(
            source_live_len,
            events,
            checkpoint.proposal(),
            Some(checkpoint),
            checkpoint.actor(),
            checkpoint.authority(),
            checkpoint.root(),
        )
    }

    pub(super) fn record_snapshot(
        &mut self,
        source_live_len: usize,
        events: &[Option<Event>],
        snapshot: EventArchiveSnapshot,
    ) -> Option<()> {
        self.record(
            source_live_len,
            events,
            snapshot.proposal(),
            None,
            snapshot.actor(),
            snapshot.authority(),
            snapshot.root(),
        )
    }

    fn record(
        &mut self,
        source_live_len: usize,
        events: &[Option<Event>],
        proposal: EventArchiveProposal,
        checkpoint: Option<EventArchiveCheckpoint>,
        actor: AgentId,
        authority: CapabilityId,
        root: ResourceId,
    ) -> Option<()> {
        if !self.can_record(events.len())
            || events.iter().any(Option::is_none)
            || proposal.count() != events.len()
            || proposal.first_sequence() != events.first()?.as_ref()?.sequence
            || proposal.through_sequence() != events.last()?.as_ref()?.sequence
            || checkpoint.is_some_and(|checkpoint| checkpoint.proposal() != proposal)
        {
            return None;
        }
        for (index, event) in events.iter().copied().enumerate() {
            self.events[index] = event;
        }
        self.len = events.len();
        self.source_live_len = source_live_len;
        self.proposal = Some(proposal);
        self.checkpoint = checkpoint;
        self.actor = Some(actor);
        self.authority = Some(authority);
        self.root = Some(root);
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

    pub(crate) const fn proposal(&self) -> Option<EventArchiveProposal> {
        self.proposal
    }

    pub(crate) const fn checkpoint(&self) -> Option<EventArchiveCheckpoint> {
        self.checkpoint
    }

    pub(crate) const fn actor(&self) -> Option<AgentId> {
        self.actor
    }

    pub(crate) const fn authority(&self) -> Option<CapabilityId> {
        self.authority
    }

    pub(crate) const fn root(&self) -> Option<ResourceId> {
        self.root
    }

    pub(crate) const fn is_released(&self) -> bool {
        self.checkpoint.is_some()
    }

    pub(crate) const fn is_retained_snapshot(&self) -> bool {
        self.proposal.is_some() && self.checkpoint.is_none()
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
            .record_checkpoint(source_live_len, events, checkpoint)
    }

    pub(super) fn record_event_snapshot(
        &mut self,
        source_live_len: usize,
        events: &[Option<Event>],
        snapshot: EventArchiveSnapshot,
    ) -> Option<()> {
        self.event_archive
            .record_snapshot(source_live_len, events, snapshot)
    }

    pub(crate) const fn event_archive(&self) -> &NativeEventArchive {
        &self.event_archive
    }

    pub(crate) fn into_event_archive(self) -> NativeEventArchive {
        self.event_archive
    }
}
