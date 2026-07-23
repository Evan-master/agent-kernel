#[allow(dead_code)]
mod event_archive_checkpoint_support;

use agent_kernel_core::{
    agent_image_signer_id, durable_state_signer_id, AgentId, CapabilityId, DurableAnchorMode,
    DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveManifestError,
    DurableArchiveReceipt, DurableArchiveReceiptError, DurableArchiveSignature, DurableSlot,
    DurableStateDigest, DurableStateSignerRecord, DurableStateSignerStatus, EventArchiveProposal,
    ResourceId, MAX_DURABLE_ARCHIVE_BYTES, MAX_DURABLE_ARCHIVE_EVENTS,
};
use sha2::{Digest, Sha256};

use event_archive_checkpoint_support::complete_event;

const STATE_SIGNER_DOMAIN: &[u8] = b"AGENT-KERNEL-DURABLE-STATE-SIGNER-V1\0";

fn manifest(anchor: DurableArchiveAnchor) -> DurableArchiveManifest {
    let event = complete_event();
    let proposal = EventArchiveProposal::from_segment(None, &[event]).unwrap();
    DurableArchiveManifest::new(
        proposal,
        AgentId::new(1),
        CapabilityId::new(2),
        ResourceId::new(3),
        ResourceId::new(4),
        4096,
        DurableStateDigest::from_archive(proposal.digest()),
        durable_state_signer_id([0x5a; 32]),
        7,
        anchor,
    )
    .unwrap()
}

#[test]
fn state_signer_identity_and_policy_are_separate_from_agent_images() {
    let public_key = [0x41; 32];
    let mut expected = Sha256::new();
    expected.update(STATE_SIGNER_DOMAIN);
    expected.update(public_key);
    let expected: [u8; 32] = expected.finalize().into();

    let signer_id = durable_state_signer_id(public_key);
    assert_eq!(signer_id.bytes(), expected);
    assert_ne!(signer_id.bytes(), agent_image_signer_id(public_key).bytes());
    assert!(!signer_id.is_zero());

    let root = ResourceId::new(9);
    let record =
        DurableStateSignerRecord::new(root, public_key, DurableStateSignerStatus::Active, 4)
            .unwrap();
    assert_eq!(record.signer_id, signer_id);
    assert_eq!(record.root, root);
    assert_eq!(record.public_key, public_key);
    assert_eq!(record.generation, 4);
    assert!(record.allows(root, 4));
    assert!(!record.allows(ResourceId::new(8), 4));
    assert!(!record.allows(root, 3));
    assert!(DurableStateSignerRecord::new(
        ResourceId::new(0),
        public_key,
        DurableStateSignerStatus::Active,
        4,
    )
    .is_none());
}

#[test]
fn manifest_binds_the_archive_authority_storage_signer_and_anchor() {
    let anchor = DurableArchiveAnchor::unanchored();
    let manifest = manifest(anchor);

    assert_eq!(manifest.generation(), 1);
    assert_eq!(manifest.first_sequence(), 1);
    assert_eq!(manifest.through_sequence(), 1);
    assert_eq!(manifest.event_count(), 1);
    assert_eq!(manifest.actor(), AgentId::new(1));
    assert_eq!(manifest.archive_authority(), CapabilityId::new(2));
    assert_eq!(manifest.root(), ResourceId::new(3));
    assert_eq!(manifest.storage(), ResourceId::new(4));
    assert_eq!(manifest.payload_length(), 4096);
    assert_eq!(
        manifest.payload_digest().bytes(),
        manifest.archive_digest().bytes
    );
    assert_eq!(manifest.signer_policy_generation(), 7);
    assert_eq!(manifest.anchor(), anchor);
    assert_eq!(manifest.anchor().mode(), DurableAnchorMode::Unanchored);
}

#[test]
fn manifest_rejects_unbounded_or_inconsistent_values() {
    let event = complete_event();
    let proposal = EventArchiveProposal::from_segment(None, &[event]).unwrap();
    let signer = durable_state_signer_id([0x5a; 32]);
    let anchor = DurableArchiveAnchor::unanchored();

    assert_eq!(
        DurableArchiveManifest::new(
            proposal,
            AgentId::new(1),
            CapabilityId::new(2),
            ResourceId::new(3),
            ResourceId::new(4),
            0,
            DurableStateDigest::from_archive(proposal.digest()),
            signer,
            7,
            anchor,
        ),
        Err(DurableArchiveManifestError::PayloadLengthOutOfRange {
            length: 0,
            limit: MAX_DURABLE_ARCHIVE_BYTES as u32,
        })
    );
    assert_eq!(
        DurableArchiveManifest::new(
            proposal,
            AgentId::new(1),
            CapabilityId::new(2),
            ResourceId::new(3),
            ResourceId::new(4),
            4096,
            DurableStateDigest::new([0x77; 32]),
            signer,
            7,
            anchor,
        ),
        Err(DurableArchiveManifestError::PayloadDigestMismatch)
    );

    let mut events = [complete_event(); MAX_DURABLE_ARCHIVE_EVENTS + 1];
    for (index, event) in events.iter_mut().enumerate() {
        event.sequence = index as u64 + 1;
    }
    let oversized = EventArchiveProposal::from_segment(None, &events).unwrap();
    assert_eq!(
        DurableArchiveManifest::new(
            oversized,
            AgentId::new(1),
            CapabilityId::new(2),
            ResourceId::new(3),
            ResourceId::new(4),
            4096,
            DurableStateDigest::from_archive(oversized.digest()),
            signer,
            7,
            anchor,
        ),
        Err(DurableArchiveManifestError::EventCountOutOfRange {
            count: MAX_DURABLE_ARCHIVE_EVENTS + 1,
            limit: MAX_DURABLE_ARCHIVE_EVENTS,
        })
    );
}

#[test]
fn slot_signature_and_receipt_are_fixed_and_generation_bound() {
    assert_eq!(DurableSlot::for_generation(0), None);
    assert_eq!(DurableSlot::for_generation(1), Some(DurableSlot::A));
    assert_eq!(DurableSlot::for_generation(2), Some(DurableSlot::B));
    assert_eq!(DurableSlot::for_generation(3), Some(DurableSlot::A));
    assert_eq!(DurableSlot::A.alternate(), DurableSlot::B);

    let signature = DurableArchiveSignature::new([0x91; 64]);
    assert_eq!(signature.bytes(), [0x91; 64]);
    assert_eq!(core::mem::size_of::<DurableArchiveSignature>(), 64);

    let manifest = manifest(DurableArchiveAnchor::unanchored());
    let manifest_digest = DurableStateDigest::new([0x21; 32]);
    let receipt = DurableArchiveReceipt::new(
        DurableSlot::A,
        manifest.storage(),
        manifest.generation(),
        manifest.archive_digest(),
        manifest_digest,
        DurableStateDigest::new([0x22; 32]),
        5,
        manifest.anchor(),
    )
    .unwrap();

    assert_eq!(receipt.slot(), DurableSlot::A);
    assert_eq!(receipt.flush_epoch(), 5);
    assert!(receipt.matches(manifest, manifest_digest));
    assert_eq!(
        DurableArchiveReceipt::new(
            DurableSlot::B,
            manifest.storage(),
            manifest.generation(),
            manifest.archive_digest(),
            manifest_digest,
            DurableStateDigest::new([0x22; 32]),
            5,
            manifest.anchor(),
        ),
        Err(DurableArchiveReceiptError::SlotGenerationMismatch {
            expected: DurableSlot::A,
            actual: DurableSlot::B,
        })
    );
}
