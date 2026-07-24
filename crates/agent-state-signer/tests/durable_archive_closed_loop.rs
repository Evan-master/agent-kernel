mod closed_loop_support;

use agent_kernel_core::{
    durable_state_signer_id, durable_state_signer_id_for_key, DurableArchiveManifestVersion,
    DurableArchiveSignature, DurableSignatureAlgorithm, DurableStatePublicKey,
    DurableStateSignerId, DurableStateSignerRecord, DurableStateSignerStatus,
    DURABLE_ARCHIVE_MANIFEST_BYTES, MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES,
};
use agent_kernel_hal::DURABLE_SLOT_BYTES;
use agent_kernel_x86_64::ata::{
    AtaDeviceIdentity, AtaDrive, AtaPioConfig, NativeAtaDurableConfig, NativeDurableArchiveCaller,
};
use agent_state_signer::{StateSignerAgent, StateSignerPolicy, StateSignerProvider};
use ed25519_dalek::{Signer, SigningKey};
use p256::ecdsa::{Signature as P256Signature, SigningKey as P256SigningKey};

use closed_loop_support::{launched_archive_kernel, ArchiveKernel, SectorDevice};

const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;
const POLICY_GENERATION: u64 = 23;
const CALL_DATA_GENERATION: u64 = 29;

struct Ed25519Provider(SigningKey);

impl StateSignerProvider for Ed25519Provider {
    type Error = ();

    fn signature_algorithm(&self) -> DurableSignatureAlgorithm {
        DurableSignatureAlgorithm::Ed25519
    }

    fn signer_id(&self) -> DurableStateSignerId {
        durable_state_signer_id(self.0.verifying_key().to_bytes())
    }

    fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, Self::Error> {
        Ok(DurableArchiveSignature::new(
            self.0.sign(manifest).to_bytes(),
        ))
    }
}

struct P256Provider {
    key: P256SigningKey,
    signer_id: DurableStateSignerId,
}

impl StateSignerProvider for P256Provider {
    type Error = ();

    fn signature_algorithm(&self) -> DurableSignatureAlgorithm {
        DurableSignatureAlgorithm::EcdsaP256Sha256
    }

    fn signer_id(&self) -> DurableStateSignerId {
        self.signer_id
    }

    fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, Self::Error> {
        let signature: P256Signature = self.key.sign(manifest);
        let signature = signature.normalize_s().unwrap_or(signature);
        Ok(DurableArchiveSignature::new(signature.to_bytes().into()))
    }
}

#[test]
fn signed_archive_survives_power_loss_and_recovers_into_virgin_core() {
    let mut fixture = launched_archive_kernel();
    let proposal = fixture
        .kernel
        .sys_prepare_event_archive(fixture.kernel.events()[3].sequence)
        .unwrap();
    let preflight = fixture
        .kernel
        .preflight_durable_event_archive(
            fixture.actor,
            fixture.archive_authority,
            fixture.storage_authority,
            fixture.storage,
            proposal,
        )
        .unwrap();
    let caller =
        NativeDurableArchiveCaller::new(fixture.actor, fixture.task, fixture.image).unwrap();
    let key = SigningKey::from_bytes(&[0xd4; 32]);
    let signer_id = durable_state_signer_id(key.verifying_key().to_bytes());
    let signer = DurableStateSignerRecord::new(
        fixture.root,
        key.verifying_key().to_bytes(),
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    )
    .unwrap();
    let config = NativeAtaDurableConfig::new(
        AtaPioConfig::new(0x170, 0x376, AtaDrive::Master, 10_000).unwrap(),
        fixture.root,
        fixture.storage,
        BASE_LBA,
        signer,
        POLICY_GENERATION,
    )
    .unwrap();
    let mut old_buffers = buffers();
    let mut session = config
        .initialize_device(
            SectorDevice::new(AtaDeviceIdentity::new(DEVICE_SECTORS).unwrap()),
            old_buffers.0.as_mut(),
            old_buffers.1.as_mut(),
            old_buffers.2.as_mut(),
        )
        .unwrap();
    let preparation = session
        .prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION,
        )
        .unwrap();
    let mut request_bytes = preparation.request_bytes();
    let mut signer_agent = StateSignerAgent::new(
        StateSignerPolicy::new(fixture.root, fixture.storage, signer_id, POLICY_GENERATION)
            .unwrap(),
        Ed25519Provider(key),
    );
    signer_agent
        .sign_prepared_request(&mut request_bytes, CALL_DATA_GENERATION)
        .unwrap();

    let mut verified = session
        .commit_prepared(caller, preflight, &request_bytes)
        .unwrap();
    let checkpoint = fixture
        .kernel
        .commit_verified_event_archive(
            fixture.actor,
            fixture.archive_authority,
            fixture.storage_authority,
            proposal,
            verified.receipt(),
            &mut verified,
        )
        .unwrap();
    assert!(verified.is_consumed());

    let mut device = session.into_device();
    device.simulate_power_loss();
    let mut recovered_buffers = buffers();
    let mut recovered_session = config
        .initialize_device(
            device,
            recovered_buffers.0.as_mut(),
            recovered_buffers.1.as_mut(),
            recovered_buffers.2.as_mut(),
        )
        .unwrap();
    let head = recovered_session.recovered_head().unwrap();
    assert_eq!(head.generation(), checkpoint.generation());
    assert_eq!(head.archive_digest(), checkpoint.digest());

    let mut recovered_kernel = ArchiveKernel::new();
    let recovery_verifier = recovered_session.recovery_verifier_mut().unwrap();
    let recovered_checkpoint = recovered_kernel
        .recover_verified_event_archive(head, recovery_verifier)
        .unwrap();
    assert_eq!(recovered_checkpoint, checkpoint);
    assert!(recovery_verifier.is_consumed());
    assert_eq!(
        recovered_kernel.durable_archive_receipt(),
        Some(head.receipt())
    );
}

#[test]
fn p256_archive_survives_power_loss_and_recovers_into_virgin_core() {
    let mut fixture = launched_archive_kernel();
    let proposal = fixture
        .kernel
        .sys_prepare_event_archive(fixture.kernel.events()[3].sequence)
        .unwrap();
    let preflight = fixture
        .kernel
        .preflight_durable_event_archive(
            fixture.actor,
            fixture.archive_authority,
            fixture.storage_authority,
            fixture.storage,
            proposal,
        )
        .unwrap();
    let caller =
        NativeDurableArchiveCaller::new(fixture.actor, fixture.task, fixture.image).unwrap();
    let key = P256SigningKey::from_slice(&[0xd6; 32]).unwrap();
    let encoded_key = key.verifying_key().to_encoded_point(true);
    let mut public_key_bytes = [0; 33];
    public_key_bytes.copy_from_slice(encoded_key.as_bytes());
    let public_key = DurableStatePublicKey::ecdsa_p256(public_key_bytes).unwrap();
    let signer_id = durable_state_signer_id_for_key(public_key);
    let signer = DurableStateSignerRecord::new_with_key(
        fixture.root,
        public_key,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    )
    .unwrap();
    let config = NativeAtaDurableConfig::new(
        AtaPioConfig::new(0x170, 0x376, AtaDrive::Master, 10_000).unwrap(),
        fixture.root,
        fixture.storage,
        BASE_LBA,
        signer,
        POLICY_GENERATION,
    )
    .unwrap();
    let mut old_buffers = buffers();
    let mut session = config
        .initialize_device(
            SectorDevice::new(AtaDeviceIdentity::new(DEVICE_SECTORS).unwrap()),
            old_buffers.0.as_mut(),
            old_buffers.1.as_mut(),
            old_buffers.2.as_mut(),
        )
        .unwrap();
    let preparation = session
        .prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION,
        )
        .unwrap();
    assert_eq!(
        preparation.manifest().version(),
        DurableArchiveManifestVersion::AlgorithmBound
    );
    let mut request_bytes = preparation.request_bytes();
    let policy = StateSignerPolicy::new_with_algorithm(
        fixture.root,
        fixture.storage,
        signer_id,
        DurableSignatureAlgorithm::EcdsaP256Sha256,
        POLICY_GENERATION,
    )
    .unwrap();
    let mut signer_agent = StateSignerAgent::new(policy, P256Provider { key, signer_id });
    signer_agent
        .sign_prepared_request(&mut request_bytes, CALL_DATA_GENERATION)
        .unwrap();

    let mut verified = session
        .commit_prepared(caller, preflight, &request_bytes)
        .unwrap();
    let checkpoint = fixture
        .kernel
        .commit_verified_event_archive(
            fixture.actor,
            fixture.archive_authority,
            fixture.storage_authority,
            proposal,
            verified.receipt(),
            &mut verified,
        )
        .unwrap();

    let mut device = session.into_device();
    device.simulate_power_loss();
    let mut recovered_buffers = buffers();
    let mut recovered_session = config
        .initialize_device(
            device,
            recovered_buffers.0.as_mut(),
            recovered_buffers.1.as_mut(),
            recovered_buffers.2.as_mut(),
        )
        .unwrap();
    let head = recovered_session.recovered_head().unwrap();
    assert_eq!(head.generation(), checkpoint.generation());
    assert_eq!(head.archive_digest(), checkpoint.digest());

    let mut recovered_kernel = ArchiveKernel::new();
    let recovery_verifier = recovered_session.recovery_verifier_mut().unwrap();
    let recovered_checkpoint = recovered_kernel
        .recover_verified_event_archive(head, recovery_verifier)
        .unwrap();
    assert_eq!(recovered_checkpoint, checkpoint);
    assert!(recovery_verifier.is_consumed());
}

type Buffers = (
    Box<[u8; DURABLE_SLOT_BYTES]>,
    Box<[u8; DURABLE_SLOT_BYTES]>,
    Box<[u8; MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES]>,
);

fn buffers() -> Buffers {
    (
        Box::new([0; DURABLE_SLOT_BYTES]),
        Box::new([0; DURABLE_SLOT_BYTES]),
        Box::new([0; MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES]),
    )
}
