mod event_archive_checkpoint_support;

use agent_kernel_core::{
    encode_event_archive_payload, AgentEntryKind, EventArchiveEncodingError, EventArchiveProposal,
    MAX_DURABLE_ARCHIVE_EVENTS,
};
use sha2::{Digest, Sha256};

use event_archive_checkpoint_support::{commit, complete_event, emit, fixture};

#[test]
fn canonical_payload_is_the_archive_digest_preimage() {
    let event = complete_event();
    let events = [event];
    let proposal = EventArchiveProposal::from_segment(None, &events).unwrap();
    let mut output = [0xa5; 4096];

    let encoded = encode_event_archive_payload(proposal, &events, &mut output).unwrap();
    let digest: [u8; 32] = Sha256::digest(&output[..encoded]).into();

    assert_eq!(digest, proposal.digest().bytes);
    assert!(output[encoded..].iter().all(|byte| *byte == 0xa5));
}

#[test]
fn chained_payload_commits_the_previous_archive_head() {
    let (mut core, fixture) = fixture::<32>(AgentEntryKind::Supervisor);
    let first_through = core.events().last().unwrap().sequence;
    let first = core.prepare_event_archive(first_through).unwrap();
    let first_checkpoint = commit(&mut core, fixture, first).unwrap();
    emit(&mut core, fixture, 77);
    let second_through = core.events().last().unwrap().sequence;
    let second = core.prepare_event_archive(second_through).unwrap();
    let mut output = [0; 4096];

    let encoded = encode_event_archive_payload(second, core.events(), &mut output).unwrap();
    let digest: [u8; 32] = Sha256::digest(&output[..encoded]).into();

    assert_eq!(second.generation(), first_checkpoint.generation() + 1);
    assert_eq!(second.previous_digest(), first_checkpoint.digest());
    assert_eq!(digest, second.digest().bytes);
}

#[test]
fn encoding_failures_leave_the_destination_unchanged() {
    let event = complete_event();
    let events = [event];
    let proposal = EventArchiveProposal::from_segment(None, &events).unwrap();
    let mut complete = [0; 4096];
    let required = encode_event_archive_payload(proposal, &events, &mut complete).unwrap();

    let mut short = vec![0x5a; required - 1];
    let unchanged = short.clone();
    assert_eq!(
        encode_event_archive_payload(proposal, &events, &mut short),
        Err(EventArchiveEncodingError::BufferTooSmall {
            required,
            available: required - 1,
        })
    );
    assert_eq!(short, unchanged);

    let mut altered = event;
    altered.agent = agent_kernel_core::AgentId::new(99);
    let mut destination = [0x3c; 4096];
    assert_eq!(
        encode_event_archive_payload(proposal, &[altered], &mut destination),
        Err(EventArchiveEncodingError::ProposalMismatch)
    );
    assert!(destination.iter().all(|byte| *byte == 0x3c));
}

#[test]
fn durable_archive_segment_has_a_fixed_event_bound() {
    let mut events = [complete_event(); MAX_DURABLE_ARCHIVE_EVENTS + 1];
    for (index, event) in events.iter_mut().enumerate() {
        event.sequence = index as u64 + 1;
    }
    let proposal = EventArchiveProposal::from_segment(None, &events).unwrap();
    let mut output = [0; 64 * 1024];

    assert_eq!(
        encode_event_archive_payload(proposal, &events, &mut output),
        Err(EventArchiveEncodingError::EventCountExceeded {
            count: MAX_DURABLE_ARCHIVE_EVENTS + 1,
            limit: MAX_DURABLE_ARCHIVE_EVENTS,
        })
    );
}
