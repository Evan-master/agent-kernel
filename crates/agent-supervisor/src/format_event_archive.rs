//! Host-side formatting for Event archive chain heads.

use std::fmt::Write;

use agent_kernel_core::{
    DurableArchiveReceipt, DurableSlot, DurableStateDigest, EventArchiveCheckpoint,
    EventArchiveDigest,
};

pub(crate) fn format_event_archive_checkpoint(checkpoint: &EventArchiveCheckpoint) -> String {
    format!(
        "event_archive_checkpoint generation={} first={} through={} count={} actor={} authority={} root={} previous_digest={} digest={}",
        checkpoint.generation(),
        checkpoint.first_sequence(),
        checkpoint.through_sequence(),
        checkpoint.count(),
        checkpoint.actor().raw(),
        checkpoint.authority().raw(),
        checkpoint.root().raw(),
        format_digest(checkpoint.previous_digest()),
        format_digest(checkpoint.digest()),
    )
}

pub(crate) fn format_durable_archive_receipt(receipt: &DurableArchiveReceipt) -> String {
    let slot = match receipt.slot() {
        DurableSlot::A => "A",
        DurableSlot::B => "B",
    };
    format!(
        "durable_archive_receipt slot={slot} storage={} generation={} archive_digest={} manifest_digest={} readback_digest={} flush_epoch={}",
        receipt.storage().raw(),
        receipt.generation(),
        format_digest(receipt.archive_digest()),
        format_state_digest(receipt.manifest_digest()),
        format_state_digest(receipt.readback_digest()),
        receipt.flush_epoch(),
    )
}

fn format_digest(digest: EventArchiveDigest) -> String {
    let mut output = String::with_capacity(digest.bytes.len() * 2);
    for byte in digest.bytes {
        write!(&mut output, "{byte:02x}").expect("writing into a String cannot fail");
    }
    output
}

fn format_state_digest(digest: DurableStateDigest) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest.bytes() {
        write!(&mut output, "{byte:02x}").expect("writing into a String cannot fail");
    }
    output
}
