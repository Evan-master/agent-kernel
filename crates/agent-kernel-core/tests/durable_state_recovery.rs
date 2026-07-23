#[allow(dead_code)]
mod event_archive_checkpoint_support;

use agent_kernel_core::{
    durable_state_signer_id, AgentId, CapabilityId, DurableArchiveAnchor, DurableArchiveManifest,
    DurableArchiveReceipt, DurableRecoveredHead, DurableRecoveryGuarantee, DurableSlot,
    DurableStateDigest, EventArchiveDigest, EventArchiveProposal, ResourceId,
};

use event_archive_checkpoint_support::complete_event;

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

fn receipt(manifest: DurableArchiveManifest, seed: u8) -> DurableArchiveReceipt {
    DurableArchiveReceipt::new(
        DurableSlot::A,
        manifest.storage(),
        manifest.generation(),
        manifest.archive_digest(),
        DurableStateDigest::new([seed; 32]),
        DurableStateDigest::new([seed + 1; 32]),
        seed as u64,
        manifest.anchor(),
    )
    .unwrap()
}

#[test]
fn recovered_heads_report_their_actual_rollback_guarantee() {
    let unanchored_manifest = manifest(DurableArchiveAnchor::unanchored());
    let unanchored =
        DurableRecoveredHead::from_verified(unanchored_manifest, receipt(unanchored_manifest, 1))
            .unwrap();
    assert_eq!(
        unanchored.guarantee(),
        DurableRecoveryGuarantee::RollbackEvident
    );

    let trusted_genesis = DurableArchiveAnchor::trusted(0, EventArchiveDigest::ZERO).unwrap();
    let anchored_manifest = manifest(trusted_genesis);
    let anchored =
        DurableRecoveredHead::from_verified(anchored_manifest, receipt(anchored_manifest, 2))
            .unwrap();
    assert_eq!(
        anchored.guarantee(),
        DurableRecoveryGuarantee::RollbackResistant
    );
    assert_eq!(
        anchored.previous_digest(),
        anchored_manifest.previous_digest()
    );
    assert_eq!(anchored.through_sequence(), 1);
}

#[test]
fn trusted_anchor_generation_and_digest_are_one_consistent_value() {
    assert_eq!(
        DurableArchiveAnchor::trusted(0, EventArchiveDigest::new([0x11; 32])),
        None
    );
    assert_eq!(
        DurableArchiveAnchor::trusted(1, EventArchiveDigest::ZERO),
        None
    );
    assert!(DurableArchiveAnchor::trusted(1, EventArchiveDigest::new([0x22; 32])).is_some());
}
