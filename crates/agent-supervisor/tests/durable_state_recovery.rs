use agent_kernel_core::{
    durable_state_signer_id, encode_event_archive_payload, AgentId, CapabilityId,
    DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveManifestFields,
    DurableArchiveSignature, DurableRecoveryError, DurableRecoveryGuarantee, DurableSlot,
    DurableStateDigest, DurableStateSignerRecord, DurableStateSignerStatus, EventArchiveDigest,
    EventArchiveProposal, KernelCore, ResourceId,
};
use agent_kernel_hal::{DurableStateBackendError, DURABLE_SLOT_BYTES};
use agent_kernel_x86_64::durable_state::{
    commit_durable_archive, encode_durable_archive_manifest, recover_durable_archive,
    DurableArchiveCommitError, DurableArchiveRecoveryError, DurableStateTrustPolicy,
};
use agent_supervisor::durable_state_backend::{
    InMemoryDurableSlotPhase, InMemoryDurableStateBackend,
};
use ed25519_dalek::{Signer, SigningKey};

type RecoveryCore = KernelCore<1, 0, 0, 4, 0, 0, 0, 0, 0, 0>;

const ROOT: ResourceId = ResourceId::new(3);
const STORAGE: ResourceId = ResourceId::new(4);
const POLICY_GENERATION: u64 = 7;

#[derive(Clone)]
struct SignedArchive {
    payload: Vec<u8>,
    manifest: DurableArchiveManifest,
    signature: DurableArchiveSignature,
    signer: DurableStateSignerRecord,
}

fn genesis() -> SignedArchive {
    let signing_key = signing_key();
    let mut core = RecoveryCore::new();
    core.register_agent(AgentId::new(1)).unwrap();
    let proposal = EventArchiveProposal::from_segment(None, core.events()).unwrap();
    let mut payload = vec![0; 4096];
    let payload_length =
        encode_event_archive_payload(proposal, core.events(), &mut payload).unwrap();
    payload.truncate(payload_length);
    let signer = DurableStateSignerRecord::new(
        ROOT,
        signing_key.verifying_key().to_bytes(),
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    )
    .unwrap();
    let manifest = DurableArchiveManifest::new(
        proposal,
        AgentId::new(1),
        CapabilityId::new(2),
        ROOT,
        STORAGE,
        payload_length as u32,
        DurableStateDigest::from_archive(proposal.digest()),
        durable_state_signer_id(signing_key.verifying_key().to_bytes()),
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap();
    SignedArchive {
        payload,
        manifest,
        signature: sign(manifest),
        signer,
    }
}

fn successor(
    base: &SignedArchive,
    generation: u64,
    previous_digest: EventArchiveDigest,
    anchor: DurableArchiveAnchor,
) -> SignedArchive {
    let fields = base.manifest.fields();
    let manifest = DurableArchiveManifest::from_fields(DurableArchiveManifestFields {
        generation,
        first_sequence: fields.through_sequence + 1,
        through_sequence: fields.through_sequence + 1,
        previous_digest,
        anchor,
        ..fields
    })
    .unwrap();
    SignedArchive {
        payload: base.payload.clone(),
        manifest,
        signature: sign(manifest),
        signer: base.signer,
    }
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[0x61; 32])
}

fn sign(manifest: DurableArchiveManifest) -> DurableArchiveSignature {
    DurableArchiveSignature::new(
        signing_key()
            .sign(&encode_durable_archive_manifest(manifest))
            .to_bytes(),
    )
}

fn commit(
    backend: &mut InMemoryDurableStateBackend,
    archive: &SignedArchive,
) -> Result<(), DurableArchiveCommitError> {
    let policy =
        DurableStateTrustPolicy::new(core::slice::from_ref(&archive.signer), POLICY_GENERATION);
    let mut scratch = vec![0; DURABLE_SLOT_BYTES];
    commit_durable_archive(
        backend,
        policy,
        &archive.payload,
        archive.manifest,
        archive.signature,
        &mut scratch,
    )?;
    Ok(())
}

fn recover(
    backend: &mut InMemoryDurableStateBackend,
    signer: &DurableStateSignerRecord,
) -> Result<agent_kernel_core::DurableRecoveredHead, DurableArchiveRecoveryError> {
    let policy = DurableStateTrustPolicy::new(core::slice::from_ref(signer), POLICY_GENERATION);
    let mut scratch = vec![0; DURABLE_SLOT_BYTES];
    recover_durable_archive(backend, policy, STORAGE, &mut scratch)
}

#[test]
fn recovery_selects_the_highest_connected_committed_generation() {
    let first = genesis();
    let second = successor(
        &first,
        2,
        first.manifest.archive_digest(),
        DurableArchiveAnchor::unanchored(),
    );
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    commit(&mut backend, &first).unwrap();
    commit(&mut backend, &second).unwrap();

    let head = recover(&mut backend, &first.signer).unwrap();

    assert_eq!(head.generation(), 2);
    assert_eq!(head.slot(), DurableSlot::B);
    assert_eq!(head.manifest(), second.manifest);
    assert_eq!(head.guarantee(), DurableRecoveryGuarantee::RollbackEvident);
    assert_eq!(backend.operation_count(), 18);
}

#[test]
fn recovery_observes_only_the_last_flushed_commit_boundary() {
    for (interrupted_operation, expected_generation, expected_phase) in [
        (7, 1, InMemoryDurableSlotPhase::Body),
        (8, 2, InMemoryDurableSlotPhase::Committed),
    ] {
        let first = genesis();
        let second = successor(
            &first,
            2,
            first.manifest.archive_digest(),
            DurableArchiveAnchor::unanchored(),
        );
        let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
        commit(&mut backend, &first).unwrap();
        backend
            .inject_interrupt_after(interrupted_operation)
            .unwrap();

        assert_eq!(
            commit(&mut backend, &second),
            Err(DurableArchiveCommitError::Backend(
                DurableStateBackendError::Interrupted
            ))
        );
        assert_eq!(backend.durable_phase(DurableSlot::B), expected_phase);
        assert_eq!(
            recover(&mut backend, &first.signer).unwrap().generation(),
            expected_generation
        );
    }
}

#[test]
fn trusted_anchor_recovers_without_a_local_predecessor_slot() {
    let first = genesis();
    let anchor = DurableArchiveAnchor::trusted(1, first.manifest.archive_digest()).unwrap();
    let second = successor(&first, 2, first.manifest.archive_digest(), anchor);
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    commit(&mut backend, &second).unwrap();

    let head = recover(&mut backend, &first.signer).unwrap();

    assert_eq!(head.generation(), 2);
    assert_eq!(
        head.guarantee(),
        DurableRecoveryGuarantee::RollbackResistant
    );
}

#[test]
fn recovery_rejects_disconnected_and_anchor_divergent_heads() {
    let first = genesis();
    let disconnected = successor(
        &first,
        4,
        EventArchiveDigest::new([0x81; 32]),
        DurableArchiveAnchor::unanchored(),
    );
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    commit(&mut backend, &first).unwrap();
    commit(&mut backend, &disconnected).unwrap();
    assert_eq!(
        recover(&mut backend, &first.signer),
        Err(DurableArchiveRecoveryError::Selection(
            DurableRecoveryError::DisconnectedHead { generation: 4 }
        ))
    );

    let anchor_digest = EventArchiveDigest::new([0x82; 32]);
    let divergent = successor(
        &first,
        2,
        anchor_digest,
        DurableArchiveAnchor::trusted(1, anchor_digest).unwrap(),
    );
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    commit(&mut backend, &first).unwrap();
    commit(&mut backend, &divergent).unwrap();
    assert_eq!(
        recover(&mut backend, &first.signer),
        Err(DurableArchiveRecoveryError::Selection(
            DurableRecoveryError::AnchorMismatch { generation: 2 }
        ))
    );
}
