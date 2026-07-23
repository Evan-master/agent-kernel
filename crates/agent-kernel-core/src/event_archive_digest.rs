//! Canonical SHA-256 commitment for archived Event segments.
//!
//! This Core implementation routes one frozen byte format into hash, counting,
//! and bounded output sinks. Field order lives in the `fields` child while
//! stable enum tags live in `tags`.

mod fields;
mod tags;

use sha2::{Digest, Sha256};

use crate::{Event, EventArchiveDigest};

const DOMAIN: &[u8] = b"AGENT-KERNEL-EVENT-ARCHIVE\0";
const FORMAT_VERSION: u16 = 2;

pub(crate) trait ArchiveSink {
    fn update(&mut self, data: impl AsRef<[u8]>);
}

struct HashSink(Sha256);

impl ArchiveSink for HashSink {
    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.0.update(data);
    }
}

pub(super) fn digest(
    generation: u64,
    previous_through: u64,
    previous_digest: EventArchiveDigest,
    first: u64,
    through: u64,
    events: &[Event],
) -> EventArchiveDigest {
    let mut hash = HashSink(Sha256::new());
    encode(
        &mut hash,
        generation,
        previous_through,
        previous_digest,
        first,
        through,
        events,
    );
    let output = hash.0.finalize();
    let mut bytes = [0; 32];
    bytes.copy_from_slice(&output);
    EventArchiveDigest::new(bytes)
}

pub(crate) fn encode(
    sink: &mut impl ArchiveSink,
    generation: u64,
    previous_through: u64,
    previous_digest: EventArchiveDigest,
    first: u64,
    through: u64,
    events: &[Event],
) {
    sink.update(DOMAIN);
    sink.update(FORMAT_VERSION.to_le_bytes());
    sink.update(generation.to_le_bytes());
    sink.update(previous_through.to_le_bytes());
    sink.update(previous_digest.bytes);
    sink.update(first.to_le_bytes());
    sink.update(through.to_le_bytes());
    sink.update((events.len() as u64).to_le_bytes());
    for event in events {
        fields::put_event(sink, event);
    }
}
