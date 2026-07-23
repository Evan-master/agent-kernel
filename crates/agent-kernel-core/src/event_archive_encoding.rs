//! Bounded canonical byte encoding for durable Event Archive payloads.
//!
//! This no_std Core module reuses the archive digest field encoder through a
//! sink contract. It validates proposal identity and output capacity before
//! writing, so every failure preserves the caller's destination bytes.

use crate::{
    event_archive_digest::{self, ArchiveSink},
    Event, EventArchiveProposal,
};

pub const MAX_DURABLE_ARCHIVE_EVENTS: usize = 64;
pub const MAX_DURABLE_ARCHIVE_BYTES: usize = 64 * 1024;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EventArchiveEncodingError {
    EventCountExceeded { count: usize, limit: usize },
    ProposalMismatch,
    PayloadTooLarge { required: usize, limit: usize },
    BufferTooSmall { required: usize, available: usize },
}

pub fn encode_event_archive_payload(
    proposal: EventArchiveProposal,
    events: &[Event],
    output: &mut [u8],
) -> Result<usize, EventArchiveEncodingError> {
    if events.len() > MAX_DURABLE_ARCHIVE_EVENTS {
        return Err(EventArchiveEncodingError::EventCountExceeded {
            count: events.len(),
            limit: MAX_DURABLE_ARCHIVE_EVENTS,
        });
    }
    let previous_through = validate_proposal(proposal, events)?;
    let mut count = CountingSink { length: 0 };
    encode(&mut count, proposal, previous_through, events);
    if count.length > MAX_DURABLE_ARCHIVE_BYTES {
        return Err(EventArchiveEncodingError::PayloadTooLarge {
            required: count.length,
            limit: MAX_DURABLE_ARCHIVE_BYTES,
        });
    }
    if output.len() < count.length {
        return Err(EventArchiveEncodingError::BufferTooSmall {
            required: count.length,
            available: output.len(),
        });
    }

    let mut destination = SliceSink { output, offset: 0 };
    encode(&mut destination, proposal, previous_through, events);
    Ok(destination.offset)
}

fn validate_proposal(
    proposal: EventArchiveProposal,
    events: &[Event],
) -> Result<u64, EventArchiveEncodingError> {
    let Some(first) = events.first() else {
        return Err(EventArchiveEncodingError::ProposalMismatch);
    };
    let Some(last) = events.last() else {
        return Err(EventArchiveEncodingError::ProposalMismatch);
    };
    if proposal.count() != events.len()
        || proposal.first_sequence() != first.sequence
        || proposal.through_sequence() != last.sequence
    {
        return Err(EventArchiveEncodingError::ProposalMismatch);
    }
    let mut expected = proposal.first_sequence();
    for (index, event) in events.iter().enumerate() {
        if event.sequence != expected {
            return Err(EventArchiveEncodingError::ProposalMismatch);
        }
        if index + 1 < events.len() {
            expected = expected
                .checked_add(1)
                .ok_or(EventArchiveEncodingError::ProposalMismatch)?;
        }
    }
    let previous_through = proposal
        .first_sequence()
        .checked_sub(1)
        .ok_or(EventArchiveEncodingError::ProposalMismatch)?;
    let digest = event_archive_digest::digest(
        proposal.generation(),
        previous_through,
        proposal.previous_digest(),
        proposal.first_sequence(),
        proposal.through_sequence(),
        events,
    );
    if digest != proposal.digest() {
        return Err(EventArchiveEncodingError::ProposalMismatch);
    }
    Ok(previous_through)
}

fn encode(
    sink: &mut impl ArchiveSink,
    proposal: EventArchiveProposal,
    previous_through: u64,
    events: &[Event],
) {
    event_archive_digest::encode(
        sink,
        proposal.generation(),
        previous_through,
        proposal.previous_digest(),
        proposal.first_sequence(),
        proposal.through_sequence(),
        events,
    );
}

struct CountingSink {
    length: usize,
}

impl ArchiveSink for CountingSink {
    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.length = self.length.saturating_add(data.as_ref().len());
    }
}

struct SliceSink<'a> {
    output: &'a mut [u8],
    offset: usize,
}

impl ArchiveSink for SliceSink<'_> {
    fn update(&mut self, data: impl AsRef<[u8]>) {
        let bytes = data.as_ref();
        let end = self.offset + bytes.len();
        self.output[self.offset..end].copy_from_slice(bytes);
        self.offset = end;
    }
}
