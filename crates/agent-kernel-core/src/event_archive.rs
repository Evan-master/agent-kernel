//! Fixed-width Event archive proposals, digests, and checkpoints.
//!
//! This Core-layer module defines allocator-free values shared by canonical
//! archive hashing, authorized commit, facades, and architecture ABIs. The
//! latest checkpoint is a chain head; complete Event segments remain external.

use crate::{AgentId, CapabilityId, Event, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EventArchiveDigest {
    pub bytes: [u8; 32],
}

impl EventArchiveDigest {
    pub const ZERO: Self = Self { bytes: [0; 32] };

    pub const fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub fn from_words_le(words: [u64; 4]) -> Self {
        let mut bytes = [0; 32];
        let mut index = 0;
        while index < words.len() {
            let encoded = words[index].to_le_bytes();
            let start = index * encoded.len();
            bytes[start..start + encoded.len()].copy_from_slice(&encoded);
            index += 1;
        }
        Self { bytes }
    }

    pub fn words_le(self) -> [u64; 4] {
        let mut words = [0; 4];
        let mut index = 0;
        while index < words.len() {
            let start = index * 8;
            let mut encoded = [0; 8];
            encoded.copy_from_slice(&self.bytes[start..start + 8]);
            words[index] = u64::from_le_bytes(encoded);
            index += 1;
        }
        words
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EventArchiveProposal {
    generation: u64,
    first_sequence: u64,
    through_sequence: u64,
    count: usize,
    previous_digest: EventArchiveDigest,
    digest: EventArchiveDigest,
}

impl EventArchiveProposal {
    pub fn from_segment(
        previous: Option<EventArchiveCheckpoint>,
        events: &[Event],
    ) -> Option<Self> {
        let first = events.first()?.sequence;
        let through = events.last()?.sequence;
        let (generation, previous_through, previous_digest) = match previous {
            Some(checkpoint) => (
                checkpoint.generation().checked_add(1)?,
                checkpoint.through_sequence(),
                checkpoint.digest(),
            ),
            None => (1, 0, EventArchiveDigest::ZERO),
        };
        if first != previous_through.checked_add(1)? {
            return None;
        }
        let mut expected = first;
        for (index, event) in events.iter().enumerate() {
            if event.sequence != expected {
                return None;
            }
            if index + 1 < events.len() {
                expected = expected.checked_add(1)?;
            }
        }
        let count = events.len();
        let digest = crate::event_archive_digest::digest(
            generation,
            previous_through,
            previous_digest,
            first,
            through,
            events,
        );
        Some(Self {
            generation,
            first_sequence: first,
            through_sequence: through,
            count,
            previous_digest,
            digest,
        })
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn first_sequence(self) -> u64 {
        self.first_sequence
    }

    pub const fn through_sequence(self) -> u64 {
        self.through_sequence
    }

    pub const fn count(self) -> usize {
        self.count
    }

    pub const fn previous_digest(self) -> EventArchiveDigest {
        self.previous_digest
    }

    pub const fn digest(self) -> EventArchiveDigest {
        self.digest
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EventArchiveCheckpoint {
    proposal: EventArchiveProposal,
    actor: AgentId,
    authority: CapabilityId,
    root: ResourceId,
}

impl EventArchiveCheckpoint {
    pub(crate) const fn new(
        proposal: EventArchiveProposal,
        actor: AgentId,
        authority: CapabilityId,
        root: ResourceId,
    ) -> Self {
        Self {
            proposal,
            actor,
            authority,
            root,
        }
    }

    pub const fn proposal(self) -> EventArchiveProposal {
        self.proposal
    }

    pub const fn generation(self) -> u64 {
        self.proposal.generation()
    }

    pub const fn first_sequence(self) -> u64 {
        self.proposal.first_sequence()
    }

    pub const fn through_sequence(self) -> u64 {
        self.proposal.through_sequence()
    }

    pub const fn count(self) -> usize {
        self.proposal.count()
    }

    pub const fn previous_digest(self) -> EventArchiveDigest {
        self.proposal.previous_digest()
    }

    pub const fn digest(self) -> EventArchiveDigest {
        self.proposal.digest()
    }

    pub const fn actor(self) -> AgentId {
        self.actor
    }

    pub const fn authority(self) -> CapabilityId {
        self.authority
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }
}
