//! Signed durable Event Archive orchestration for the host Supervisor.
//!
//! Private signing material and storage selection stay in this std adapter.
//! Kernel crates receive only canonical values, a verified receipt, and the
//! one-shot machine verifier produced by the completed transaction.

use agent_kernel_core::{
    durable_state_signer_id, encode_event_archive_payload, AgentId, CapabilityId,
    DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveManifestError,
    DurableArchiveReceipt, DurableArchiveSignature, DurableStateDigest, DurableStateSignerRecord,
    DurableStateSignerStatus, EventArchiveCheckpoint, EventArchiveEncodingError,
    EventArchiveProposal, KernelError, ResourceId, MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES,
};
use agent_kernel_hal::DURABLE_SLOT_BYTES;
use agent_kernel_x86_64::durable_state::{
    commit_durable_archive, encode_durable_archive_manifest, recover_durable_archive,
    DurableArchiveCommitError, DurableArchiveRecoveryError, DurableStateTrustPolicy,
};
use agent_supervisor::durable_state_backend::InMemoryDurableStateBackend;
use ed25519_dalek::{Signer, SigningKey};

use crate::flow_resources::SupervisorKernel;

const POLICY_GENERATION: u64 = 1;
const DEMO_SIGNING_SEED: [u8; 32] = [0x73; 32];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveFlowError {
    ProposalRangeMismatch,
    Payload(EventArchiveEncodingError),
    Manifest(DurableArchiveManifestError),
    SignerPolicyInvalid,
    StorageResourceInvalid,
    Transaction(DurableArchiveCommitError),
    Recovery(DurableArchiveRecoveryError),
    RecoveredManifestMismatch,
    Kernel(KernelError),
    CommitVerifierNotConsumed,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveFlowOutcome {
    pub checkpoint: EventArchiveCheckpoint,
    pub receipt: DurableArchiveReceipt,
}

pub fn commit_signed_archive(
    kernel: &mut SupervisorKernel,
    actor: AgentId,
    archive_authority: CapabilityId,
    storage_authority: CapabilityId,
    root: ResourceId,
    storage: ResourceId,
    proposal: EventArchiveProposal,
) -> Result<DurableArchiveFlowOutcome, DurableArchiveFlowError> {
    let events = kernel
        .events()
        .get(..proposal.count())
        .ok_or(DurableArchiveFlowError::ProposalRangeMismatch)?;
    let mut payload = vec![0; MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES];
    let payload_length = encode_event_archive_payload(proposal, events, &mut payload)
        .map_err(DurableArchiveFlowError::Payload)?;
    payload.truncate(payload_length);

    let signing_key = SigningKey::from_bytes(&DEMO_SIGNING_SEED);
    let public_key = signing_key.verifying_key().to_bytes();
    let signer = DurableStateSignerRecord::new(
        root,
        public_key,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    )
    .ok_or(DurableArchiveFlowError::SignerPolicyInvalid)?;
    let manifest = DurableArchiveManifest::new(
        proposal,
        actor,
        archive_authority,
        root,
        storage,
        payload_length as u32,
        DurableStateDigest::from_archive(proposal.digest()),
        durable_state_signer_id(public_key),
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    )
    .map_err(DurableArchiveFlowError::Manifest)?;
    let signature = DurableArchiveSignature::new(
        signing_key
            .sign(&encode_durable_archive_manifest(manifest))
            .to_bytes(),
    );
    let policy = DurableStateTrustPolicy::new(core::slice::from_ref(&signer), POLICY_GENERATION);
    let mut backend = InMemoryDurableStateBackend::new(storage)
        .ok_or(DurableArchiveFlowError::StorageResourceInvalid)?;
    let mut scratch = vec![0; DURABLE_SLOT_BYTES];
    let mut verified_commit = commit_durable_archive(
        &mut backend,
        policy,
        &payload,
        manifest,
        signature,
        &mut scratch,
    )
    .map_err(DurableArchiveFlowError::Transaction)?;
    let recovered = recover_durable_archive(&mut backend, policy, storage, &mut scratch)
        .map_err(DurableArchiveFlowError::Recovery)?;
    if recovered.manifest() != manifest {
        return Err(DurableArchiveFlowError::RecoveredManifestMismatch);
    }

    let receipt = verified_commit.receipt();
    let checkpoint = kernel
        .commit_verified_event_archive(
            actor,
            archive_authority,
            storage_authority,
            proposal,
            receipt,
            &mut verified_commit,
        )
        .map_err(DurableArchiveFlowError::Kernel)?;
    if !verified_commit.is_consumed() {
        return Err(DurableArchiveFlowError::CommitVerifierNotConsumed);
    }
    Ok(DurableArchiveFlowOutcome {
        checkpoint,
        receipt,
    })
}
