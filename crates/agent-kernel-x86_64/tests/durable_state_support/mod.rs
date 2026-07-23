use agent_kernel_core::{
    durable_state_signer_id, encode_event_archive_payload, AgentId, CapabilityId,
    DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveSignature, DurableStateDigest,
    DurableStateSignerRecord, DurableStateSignerStatus, EventArchiveProposal, KernelCore,
    ResourceId,
};
use agent_kernel_x86_64::durable_state::encode_durable_archive_manifest;
use ed25519_dalek::{Signer, SigningKey};

type ManifestCore = KernelCore<1, 0, 0, 4, 0, 0, 0, 0, 0, 0>;

pub const ROOT: ResourceId = ResourceId::new(3);
pub const STORAGE: ResourceId = ResourceId::new(4);
pub const POLICY_GENERATION: u64 = 7;

pub fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

pub fn manifest(
    signing_key: &SigningKey,
    root: ResourceId,
    storage: ResourceId,
    policy_generation: u64,
    anchor: DurableArchiveAnchor,
) -> DurableArchiveManifest {
    payload_and_manifest(signing_key, root, storage, policy_generation, anchor).1
}

pub fn payload_and_manifest(
    signing_key: &SigningKey,
    root: ResourceId,
    storage: ResourceId,
    policy_generation: u64,
    anchor: DurableArchiveAnchor,
) -> (Vec<u8>, DurableArchiveManifest) {
    let mut core = ManifestCore::new();
    core.register_agent(AgentId::new(1)).unwrap();
    let proposal = EventArchiveProposal::from_segment(None, core.events()).unwrap();
    let mut payload = [0; 4096];
    let payload_length =
        encode_event_archive_payload(proposal, core.events(), &mut payload).unwrap() as u32;

    let manifest = DurableArchiveManifest::new(
        proposal,
        AgentId::new(1),
        CapabilityId::new(2),
        root,
        storage,
        payload_length,
        DurableStateDigest::from_archive(proposal.digest()),
        durable_state_signer_id(signing_key.verifying_key().to_bytes()),
        policy_generation,
        anchor,
    )
    .unwrap();
    (payload[..payload_length as usize].to_vec(), manifest)
}

pub fn signer_record(
    signing_key: &SigningKey,
    root: ResourceId,
    status: DurableStateSignerStatus,
    generation: u64,
) -> DurableStateSignerRecord {
    DurableStateSignerRecord::new(
        root,
        signing_key.verifying_key().to_bytes(),
        status,
        generation,
    )
    .unwrap()
}

pub fn signature(
    signing_key: &SigningKey,
    manifest: DurableArchiveManifest,
) -> DurableArchiveSignature {
    let bytes = encode_durable_archive_manifest(manifest);
    DurableArchiveSignature::new(signing_key.sign(&bytes).to_bytes())
}
