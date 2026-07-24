use agent_kernel_core::{
    durable_state_signer_id, AgentId, CapabilityId, DurableArchiveAnchor, DurableArchiveManifest,
    DurableArchiveSignature, DurableStateDigest, DurableStateSignerId, EventArchiveProposal,
    KernelCore, ResourceId,
};
use agent_kernel_x86_64::{
    durable_archive_request::{
        encode_unsigned_durable_archive_request, DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET,
        DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET,
    },
    durable_state::encode_durable_archive_manifest,
};
use agent_state_signer::{
    StateSignerAgent, StateSignerAgentError, StateSignerPolicy, StateSignerProvider,
};
use ed25519_dalek::{Signer, SigningKey};

const ROOT: ResourceId = ResourceId::new(1);
const STORAGE: ResourceId = ResourceId::new(2);
const STORAGE_AUTHORITY: CapabilityId = CapabilityId::new(3);
const POLICY_GENERATION: u64 = 4;
const CALL_DATA_GENERATION: u64 = 5;

struct Ed25519Provider {
    key: SigningKey,
    calls: usize,
    fails: bool,
}

impl StateSignerProvider for Ed25519Provider {
    type Error = ();

    fn signer_id(&self) -> DurableStateSignerId {
        durable_state_signer_id(self.key.verifying_key().to_bytes())
    }

    fn sign_manifest(
        &mut self,
        manifest: &[u8; agent_kernel_core::DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, Self::Error> {
        self.calls += 1;
        if self.fails {
            Err(())
        } else {
            Ok(DurableArchiveSignature::new(
                self.key.sign(manifest).to_bytes(),
            ))
        }
    }
}

#[test]
fn signer_enforces_policy_then_fills_only_the_signature_field() {
    let key = SigningKey::from_bytes(&[0xd1; 32]);
    let signer_id = durable_state_signer_id(key.verifying_key().to_bytes());
    let manifest = manifest(signer_id);
    let mut bytes =
        encode_unsigned_durable_archive_request(CALL_DATA_GENERATION, STORAGE_AUTHORITY, manifest)
            .unwrap();
    let unsigned = bytes;
    let policy =
        StateSignerPolicy::new(ROOT, STORAGE, signer_id, POLICY_GENERATION).expect("policy");
    let mut agent = StateSignerAgent::new(
        policy,
        Ed25519Provider {
            key,
            calls: 0,
            fails: false,
        },
    );

    let signed = agent
        .sign_prepared_request(&mut bytes, CALL_DATA_GENERATION)
        .expect("signed request");

    assert_eq!(agent.provider().calls, 1);
    assert_eq!(signed.manifest(), manifest);
    assert_ne!(signed.signature().bytes(), [0; 64]);
    assert_eq!(
        &bytes[..DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET],
        &unsigned[..DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET]
    );
    assert_eq!(
        &bytes[DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET..],
        &unsigned[DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET..]
    );
    key_verifies(
        agent.provider().key.verifying_key().to_bytes(),
        signed.signature(),
        manifest,
    );
}

#[test]
fn policy_mismatch_and_existing_signature_never_call_provider() {
    let key = SigningKey::from_bytes(&[0xd2; 32]);
    let signer_id = durable_state_signer_id(key.verifying_key().to_bytes());
    let manifest = manifest(signer_id);
    let canonical =
        encode_unsigned_durable_archive_request(CALL_DATA_GENERATION, STORAGE_AUTHORITY, manifest)
            .unwrap();
    let policy = StateSignerPolicy::new(
        ResourceId::new(ROOT.raw() + 1),
        STORAGE,
        signer_id,
        POLICY_GENERATION,
    )
    .unwrap();
    let mut agent = StateSignerAgent::new(
        policy,
        Ed25519Provider {
            key,
            calls: 0,
            fails: false,
        },
    );
    let mut bytes = canonical;

    assert_eq!(
        agent.sign_prepared_request(&mut bytes, CALL_DATA_GENERATION),
        Err(StateSignerAgentError::RootMismatch)
    );
    assert_eq!(agent.provider().calls, 0);
    assert_eq!(bytes, canonical);

    let key = SigningKey::from_bytes(&[0xd2; 32]);
    let policy =
        StateSignerPolicy::new(ROOT, STORAGE, signer_id, POLICY_GENERATION).expect("policy");
    let mut agent = StateSignerAgent::new(
        policy,
        Ed25519Provider {
            key,
            calls: 0,
            fails: false,
        },
    );
    bytes[DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET] = 1;
    let modified = bytes;
    assert_eq!(
        agent.sign_prepared_request(&mut bytes, CALL_DATA_GENERATION),
        Err(StateSignerAgentError::SignatureAlreadyPresent)
    );
    assert_eq!(agent.provider().calls, 0);
    assert_eq!(bytes, modified);
}

#[test]
fn provider_failure_is_atomic() {
    let key = SigningKey::from_bytes(&[0xd3; 32]);
    let signer_id = durable_state_signer_id(key.verifying_key().to_bytes());
    let manifest = manifest(signer_id);
    let mut bytes =
        encode_unsigned_durable_archive_request(CALL_DATA_GENERATION, STORAGE_AUTHORITY, manifest)
            .unwrap();
    let canonical = bytes;
    let policy =
        StateSignerPolicy::new(ROOT, STORAGE, signer_id, POLICY_GENERATION).expect("policy");
    let mut agent = StateSignerAgent::new(
        policy,
        Ed25519Provider {
            key,
            calls: 0,
            fails: true,
        },
    );

    assert_eq!(
        agent.sign_prepared_request(&mut bytes, CALL_DATA_GENERATION),
        Err(StateSignerAgentError::Provider(()))
    );
    assert_eq!(agent.provider().calls, 1);
    assert_eq!(bytes, canonical);
}

fn manifest(signer_id: DurableStateSignerId) -> DurableArchiveManifest {
    type Core = KernelCore<1, 0, 0, 2, 0, 0, 0, 0, 0, 0>;
    let mut core = Core::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).unwrap();
    let proposal = EventArchiveProposal::from_segment(None, core.events()).unwrap();
    DurableArchiveManifest::new(
        proposal,
        actor,
        CapabilityId::new(1),
        ROOT,
        STORAGE,
        128,
        DurableStateDigest::from_archive(proposal.digest()),
        signer_id,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap()
}

fn key_verifies(
    public_key: [u8; 32],
    signature: DurableArchiveSignature,
    manifest: DurableArchiveManifest,
) {
    use ed25519_dalek::Verifier;

    let key = ed25519_dalek::VerifyingKey::from_bytes(&public_key).unwrap();
    let signature = ed25519_dalek::Signature::from_bytes(&signature.bytes());
    key.verify(&encode_durable_archive_manifest(manifest), &signature)
        .unwrap();
}
