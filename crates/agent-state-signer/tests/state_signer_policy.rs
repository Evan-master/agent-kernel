use agent_kernel_core::{
    durable_state_signer_id, durable_state_signer_id_for_key, AgentId, CapabilityId,
    DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveSignature,
    DurableSignatureAlgorithm, DurableStateDigest, DurableStatePublicKey, DurableStateSignerId,
    EventArchiveProposal, KernelCore, ResourceId,
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
use p256::ecdsa::{Signature as P256Signature, SigningKey as P256SigningKey};

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

    fn signature_algorithm(&self) -> DurableSignatureAlgorithm {
        DurableSignatureAlgorithm::Ed25519
    }

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

struct P256Provider {
    key: P256SigningKey,
    calls: usize,
}

impl StateSignerProvider for P256Provider {
    type Error = ();

    fn signature_algorithm(&self) -> DurableSignatureAlgorithm {
        DurableSignatureAlgorithm::EcdsaP256Sha256
    }

    fn signer_id(&self) -> DurableStateSignerId {
        durable_state_signer_id_for_key(p256_public_key(&self.key))
    }

    fn sign_manifest(
        &mut self,
        manifest: &[u8; agent_kernel_core::DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, Self::Error> {
        self.calls += 1;
        let signature: P256Signature = self.key.sign(manifest);
        let low = signature.normalize_s().unwrap_or(signature);
        Ok(high_s_signature(DurableArchiveSignature::new(
            low.to_bytes().into(),
        )))
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
    let canonical = P256Signature::from_slice(&signed.signature().bytes()).unwrap();
    assert!(canonical.normalize_s().is_none());
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

#[test]
fn p256_provider_signs_only_an_algorithm_bound_policy_and_manifest() {
    let key = P256SigningKey::from_slice(&[0xd4; 32]).unwrap();
    let signer_id = durable_state_signer_id_for_key(p256_public_key(&key));
    let manifest = p256_manifest(signer_id);
    let mut bytes =
        encode_unsigned_durable_archive_request(CALL_DATA_GENERATION, STORAGE_AUTHORITY, manifest)
            .unwrap();
    let policy = StateSignerPolicy::new_with_algorithm(
        ROOT,
        STORAGE,
        signer_id,
        DurableSignatureAlgorithm::EcdsaP256Sha256,
        POLICY_GENERATION,
    )
    .unwrap();
    let mut agent = StateSignerAgent::new(policy, P256Provider { key, calls: 0 });

    let signed = agent
        .sign_prepared_request(&mut bytes, CALL_DATA_GENERATION)
        .unwrap();

    assert_eq!(agent.provider().calls, 1);
    assert_eq!(
        signed.manifest().signature_algorithm(),
        DurableSignatureAlgorithm::EcdsaP256Sha256
    );
    assert_ne!(signed.signature().bytes(), [0; 64]);
}

#[test]
fn algorithm_mismatch_never_calls_the_provider() {
    let key = SigningKey::from_bytes(&[0xd5; 32]);
    let signer_id = durable_state_signer_id(key.verifying_key().to_bytes());
    let manifest = DurableArchiveManifest::from_algorithm_bound_fields(
        manifest(signer_id).fields(),
        DurableSignatureAlgorithm::EcdsaP256Sha256,
    )
    .unwrap();
    let mut bytes =
        encode_unsigned_durable_archive_request(CALL_DATA_GENERATION, STORAGE_AUTHORITY, manifest)
            .unwrap();
    let original = bytes;
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

    assert_eq!(
        agent.sign_prepared_request(&mut bytes, CALL_DATA_GENERATION),
        Err(StateSignerAgentError::SignatureAlgorithmMismatch)
    );
    assert_eq!(agent.provider().calls, 0);
    assert_eq!(bytes, original);
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

fn p256_manifest(signer_id: DurableStateSignerId) -> DurableArchiveManifest {
    DurableArchiveManifest::from_algorithm_bound_fields(
        manifest(signer_id).fields(),
        DurableSignatureAlgorithm::EcdsaP256Sha256,
    )
    .unwrap()
}

fn p256_public_key(key: &P256SigningKey) -> DurableStatePublicKey {
    let encoded = key.verifying_key().to_encoded_point(true);
    let mut bytes = [0; 33];
    bytes.copy_from_slice(encoded.as_bytes());
    DurableStatePublicKey::ecdsa_p256(bytes).unwrap()
}

fn high_s_signature(signature: DurableArchiveSignature) -> DurableArchiveSignature {
    const ORDER: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2, 0xfc, 0x63,
        0x25, 0x51,
    ];
    let mut bytes = signature.bytes();
    let mut borrow = 0u16;
    for index in (0..32).rev() {
        let minuend = u16::from(ORDER[index]);
        let subtrahend = u16::from(bytes[32 + index]) + borrow;
        if minuend >= subtrahend {
            bytes[32 + index] = (minuend - subtrahend) as u8;
            borrow = 0;
        } else {
            bytes[32 + index] = (minuend + 256 - subtrahend) as u8;
            borrow = 1;
        }
    }
    DurableArchiveSignature::new(bytes)
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
