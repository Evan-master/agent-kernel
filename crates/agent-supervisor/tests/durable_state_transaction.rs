use agent_kernel_core::{
    durable_state_signer_id, encode_event_archive_payload, AgentId, CapabilityId,
    DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveSignature, DurableSlot,
    DurableStateDigest, DurableStateSignerRecord, DurableStateSignerStatus, EventArchiveProposal,
    KernelCore, ResourceId,
};
use agent_kernel_hal::{DurableStateBackend, DurableStateBackendError, DURABLE_SLOT_BYTES};
use agent_kernel_x86_64::durable_state::{
    commit_durable_archive, encode_durable_archive_manifest, parse_durable_archive_slot,
    DecodedDurableArchiveSlot, DurableArchiveCommitError, DurableStateTrustPolicy,
    DurableStateVerificationError,
};
use agent_supervisor::durable_state_backend::{
    InMemoryDurableSlotPhase, InMemoryDurableStateBackend,
};
use ed25519_dalek::{Signer, SigningKey};

type TransactionCore = KernelCore<1, 0, 0, 4, 0, 0, 0, 0, 0, 0>;

const ROOT: ResourceId = ResourceId::new(3);
const STORAGE: ResourceId = ResourceId::new(4);
const POLICY_GENERATION: u64 = 7;

struct Fixture {
    payload: Vec<u8>,
    manifest: DurableArchiveManifest,
    signature: DurableArchiveSignature,
    signer: DurableStateSignerRecord,
}

fn fixture() -> Fixture {
    let signing_key = SigningKey::from_bytes(&[0x51; 32]);
    let mut core = TransactionCore::new();
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
    let signature = DurableArchiveSignature::new(
        signing_key
            .sign(&encode_durable_archive_manifest(manifest))
            .to_bytes(),
    );
    Fixture {
        payload,
        manifest,
        signature,
        signer,
    }
}

#[test]
fn transaction_flushes_verifies_and_returns_exact_readback_receipt() {
    let fixture = fixture();
    let policy =
        DurableStateTrustPolicy::new(core::slice::from_ref(&fixture.signer), POLICY_GENERATION);
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    let mut scratch = vec![0; DURABLE_SLOT_BYTES];

    let verified_commit = commit_durable_archive(
        &mut backend,
        policy,
        &fixture.payload,
        fixture.manifest,
        fixture.signature,
        &mut scratch,
    )
    .unwrap();
    let receipt = verified_commit.receipt();

    assert_eq!(backend.operation_count(), 8);
    assert_eq!(backend.flush_epoch(), 3);
    assert_eq!(backend.active_generation(), Some(1));
    assert_eq!(receipt.flush_epoch(), 3);
    assert!(receipt.matches(
        fixture.manifest,
        agent_kernel_x86_64::durable_state::durable_archive_manifest_digest(fixture.manifest)
    ));
    assert!(!receipt.readback_digest().is_zero());
    assert!(!verified_commit.is_consumed());

    let readback = backend
        .read_slot(STORAGE, receipt.slot(), &mut scratch)
        .unwrap();
    assert_eq!(readback.flush_epoch(), receipt.flush_epoch());
    assert!(matches!(
        parse_durable_archive_slot(&scratch, STORAGE, receipt.slot()),
        Ok(DecodedDurableArchiveSlot::Committed(_))
    ));
}

#[test]
fn validation_failures_issue_no_backend_operation() {
    let fixture = fixture();
    let policy =
        DurableStateTrustPolicy::new(core::slice::from_ref(&fixture.signer), POLICY_GENERATION);
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    let mut scratch = vec![0; DURABLE_SLOT_BYTES - 1];

    assert_eq!(
        commit_durable_archive(
            &mut backend,
            policy,
            &fixture.payload,
            fixture.manifest,
            fixture.signature,
            &mut scratch,
        ),
        Err(DurableArchiveCommitError::ScratchLengthMismatch {
            length: DURABLE_SLOT_BYTES - 1,
            required: DURABLE_SLOT_BYTES,
        })
    );
    assert_eq!(backend.operation_count(), 0);

    let mut scratch = vec![0; DURABLE_SLOT_BYTES];
    assert_eq!(
        commit_durable_archive(
            &mut backend,
            policy,
            &fixture.payload,
            fixture.manifest,
            DurableArchiveSignature::new([0; 64]),
            &mut scratch,
        ),
        Err(DurableArchiveCommitError::Trust(
            DurableStateVerificationError::SignatureInvalid
        ))
    );
    assert_eq!(backend.operation_count(), 0);
}

#[test]
fn every_transaction_operation_is_an_explicit_power_loss_boundary() {
    let expectations = [
        InMemoryDurableSlotPhase::Empty,
        InMemoryDurableSlotPhase::Empty,
        InMemoryDurableSlotPhase::Prepared,
        InMemoryDurableSlotPhase::Prepared,
        InMemoryDurableSlotPhase::Body,
        InMemoryDurableSlotPhase::Body,
        InMemoryDurableSlotPhase::Body,
        InMemoryDurableSlotPhase::Committed,
    ];

    for (index, expected) in expectations.into_iter().enumerate() {
        let fixture = fixture();
        let policy =
            DurableStateTrustPolicy::new(core::slice::from_ref(&fixture.signer), POLICY_GENERATION);
        let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
        backend.inject_interrupt_after(index as u64 + 1).unwrap();
        let mut scratch = vec![0; DURABLE_SLOT_BYTES];

        assert_eq!(
            commit_durable_archive(
                &mut backend,
                policy,
                &fixture.payload,
                fixture.manifest,
                fixture.signature,
                &mut scratch,
            ),
            Err(DurableArchiveCommitError::Backend(
                DurableStateBackendError::Interrupted
            ))
        );
        assert_eq!(backend.operation_count(), index as u64 + 1);
        assert_eq!(backend.durable_phase(DurableSlot::A), expected);
        assert_eq!(
            backend.active_generation(),
            (expected == InMemoryDurableSlotPhase::Committed).then_some(1)
        );
    }
}
