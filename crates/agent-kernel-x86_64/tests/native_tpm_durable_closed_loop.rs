#[allow(dead_code)]
mod ata_block_support;
mod durable_archive_kernel_support;
#[allow(dead_code)]
mod durable_state_support;
mod tpm2_support;

use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{DurableStateSignerStatus, MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES};
use agent_kernel_hal::DURABLE_SLOT_BYTES;
use agent_kernel_x86_64::{
    ata::{
        AtaDeviceIdentity, AtaDrive, AtaDurableHead, AtaPioConfig, NativeAtaDurableConfig,
        NativeDurableArchiveCaller,
    },
    durable_archive_request::DurableArchiveRequestDecodeError,
    durable_state::encode_durable_archive_manifest,
    tpm2::{
        sign_retained_durable_request, DigestSignCommand, KernelStateSignerServiceError,
        ProvisionedTpmSigner, ProvisionedTpmSignerConfig, Sha256PcrPolicy, TpmPersistentHandle,
        TpmPolicySessionHandle,
    },
};
use p256::ecdsa::{signature::hazmat::PrehashSigner, Signature, SigningKey};
use sha2::{Digest, Sha256};

use ata_block_support::SectorDevice;
use durable_archive_kernel_support::{launched_archive_kernel, ArchiveKernelFixture};
use durable_state_support::{p256_signer_record, POLICY_GENERATION};
use tpm2_support::{
    command_success, policy_public_fixture, policy_session_response, policy_signature_response,
    public_fixture, signature_response, ScriptedTpm,
};

const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;
const CALL_DATA_GENERATION: u64 = 31;
const HANDLE: TpmPersistentHandle =
    TpmPersistentHandle::new(0x8101_0001).expect("persistent handle");
const SESSION_HANDLE: TpmPolicySessionHandle =
    TpmPolicySessionHandle::new(0x0300_0042).expect("policy session handle");
const PCR_SELECTION: [u8; 3] = [0x81, 0x08, 0];
const PCR_DIGEST: [u8; 32] = [0xa5; 32];
const TPM_NONCE: [u8; 16] = [0xc3; 16];

#[test]
fn measured_policy_tpm_signature_commits_to_ata_and_survives_cold_recovery() {
    let key = SigningKey::from_slice(&[0x71; 32]).unwrap();
    let mut fixture = launched_archive_kernel();
    let (proposal, preflight, caller) = archive_contract(&fixture);
    let signer_record = p256_signer_record(
        &key,
        fixture.root,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let config = durable_config(&fixture, signer_record);
    let mut initial_buffers = buffers();
    let mut session = config
        .initialize_device(
            SectorDevice::new(identity()),
            initial_buffers.0.as_mut(),
            initial_buffers.1.as_mut(),
            initial_buffers.2.as_mut(),
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
    let retained = preparation.request_bytes();
    let mut signer = measured_policy_signer(&key, preparation.manifest());

    let signed = sign_retained_durable_request(
        &retained,
        &retained,
        preparation.manifest(),
        CALL_DATA_GENERATION,
        &mut signer,
    )
    .unwrap();
    let transport = signer.into_transport();
    assert_eq!(transport.commands().len(), 6);
    let manifest_digest: [u8; 32] =
        Sha256::digest(encode_durable_archive_manifest(preparation.manifest())).into();
    assert_eq!(&transport.commands()[4][31..63], &manifest_digest);
    assert_eq!(&transport.commands()[2][16..48], &PCR_DIGEST);

    let mut verified = session
        .commit_prepared(caller, preflight, &signed)
        .expect("ATA commit");
    let receipt = verified.receipt();
    let checkpoint = fixture
        .kernel
        .commit_verified_event_archive(
            fixture.actor,
            fixture.archive_authority,
            fixture.storage_authority,
            proposal,
            receipt,
            &mut verified,
        )
        .expect("Core release");
    assert!(verified.is_consumed());
    assert_eq!(
        session.backend().head(),
        Some(AtaDurableHead::Recovered(proposal.generation()))
    );
    assert_eq!(checkpoint.proposal(), proposal);

    let mut device = session.into_device();
    device.simulate_power_loss();
    let mut recovered_buffers = buffers();
    let mut recovered = config
        .initialize_device(
            device,
            recovered_buffers.0.as_mut(),
            recovered_buffers.1.as_mut(),
            recovered_buffers.2.as_mut(),
        )
        .expect("cold recovery");
    let head = recovered.recovered_head().expect("recovered head");
    assert_eq!(head.generation(), proposal.generation());

    type RecoveredBoot = BootedKernel<2, 2, 4, 16, 4, 4, 0, 0, 0, 0>;
    let proof = recovered
        .recovery_verifier_mut()
        .expect("one-shot recovery proof");
    let booted = RecoveredBoot::boot_recovered(BootConfig::default(), head, proof).unwrap();
    assert!(proof.is_consumed());
    assert_eq!(
        booted.kernel().events()[0].sequence,
        head.through_sequence() + 1
    );
}

#[test]
fn stale_generation_and_wrong_tpm_key_fail_before_sign_or_ata_write() {
    let configured_key = SigningKey::from_slice(&[0x72; 32]).unwrap();
    let wrong_key = SigningKey::from_slice(&[0x73; 32]).unwrap();
    let fixture = launched_archive_kernel();
    let (proposal, preflight, caller) = archive_contract(&fixture);
    let signer_record = p256_signer_record(
        &configured_key,
        fixture.root,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let mut storage_buffers = buffers();
    let mut session = durable_config(&fixture, signer_record)
        .initialize_device(
            SectorDevice::new(identity()),
            storage_buffers.0.as_mut(),
            storage_buffers.1.as_mut(),
            storage_buffers.2.as_mut(),
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
    let baseline_io = session.backend().device().operations().len();
    let retained = preparation.request_bytes();
    let mut wrong_signer = provisioned_signer(&wrong_key, preparation.manifest());

    assert_eq!(
        sign_retained_durable_request(
            &retained,
            &retained,
            preparation.manifest(),
            CALL_DATA_GENERATION + 1,
            &mut wrong_signer,
        ),
        Err(KernelStateSignerServiceError::Request(
            DurableArchiveRequestDecodeError::GenerationMismatch {
                expected: CALL_DATA_GENERATION + 1,
                actual: CALL_DATA_GENERATION,
            }
        ))
    );
    assert_eq!(
        sign_retained_durable_request(
            &retained,
            &retained,
            preparation.manifest(),
            CALL_DATA_GENERATION,
            &mut wrong_signer,
        ),
        Err(KernelStateSignerServiceError::SignerIdentityMismatch)
    );
    assert_eq!(wrong_signer.into_transport().commands().len(), 1);
    assert_eq!(session.backend().device().operations().len(), baseline_io);
}

fn provisioned_signer(
    key: &SigningKey,
    manifest: agent_kernel_core::DurableArchiveManifest,
) -> ProvisionedTpmSigner<ScriptedTpm> {
    let fixture = public_fixture(key, 0x0004_0072);
    let digest: [u8; 32] = Sha256::digest(encode_durable_archive_manifest(manifest)).into();
    let signature: Signature = key.sign_prehash(&digest).unwrap();
    let config = ProvisionedTpmSignerConfig::new(
        HANDLE,
        DigestSignCommand::SignDigestV185,
        POLICY_GENERATION,
        fixture.name,
        fixture.compressed,
    )
    .unwrap();
    ProvisionedTpmSigner::bind(
        ScriptedTpm::new([fixture.response, signature_response(signature)]),
        config,
    )
    .unwrap()
}

fn measured_policy_signer(
    key: &SigningKey,
    manifest: agent_kernel_core::DurableArchiveManifest,
) -> ProvisionedTpmSigner<ScriptedTpm> {
    let mode = DigestSignCommand::SignDigestV185;
    let policy = Sha256PcrPolicy::new(PCR_SELECTION, PCR_DIGEST).unwrap();
    let fixture = policy_public_fixture(key, 0x0004_00b2, policy.authorization_digest(mode));
    let digest: [u8; 32] = Sha256::digest(encode_durable_archive_manifest(manifest)).into();
    let signature: Signature = key.sign_prehash(&digest).unwrap();
    let config = ProvisionedTpmSignerConfig::new_pcr_policy(
        HANDLE,
        mode,
        POLICY_GENERATION,
        fixture.name,
        fixture.compressed,
        policy,
    )
    .unwrap();
    ProvisionedTpmSigner::bind(
        ScriptedTpm::new([
            fixture.response,
            policy_session_response(SESSION_HANDLE, TPM_NONCE),
            command_success(),
            command_success(),
            policy_signature_response(signature, TPM_NONCE),
            command_success(),
        ]),
        config,
    )
    .unwrap()
}

fn archive_contract(
    fixture: &ArchiveKernelFixture,
) -> (
    agent_kernel_core::EventArchiveProposal,
    agent_kernel_core::DurableArchivePreflight,
    NativeDurableArchiveCaller,
) {
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
    (proposal, preflight, caller)
}

fn durable_config(
    fixture: &ArchiveKernelFixture,
    signer: agent_kernel_core::DurableStateSignerRecord,
) -> NativeAtaDurableConfig {
    NativeAtaDurableConfig::new(
        AtaPioConfig::new(0x170, 0x376, AtaDrive::Master, 10_000).unwrap(),
        fixture.root,
        fixture.storage,
        BASE_LBA,
        signer,
        POLICY_GENERATION,
    )
    .unwrap()
}

fn identity() -> AtaDeviceIdentity {
    AtaDeviceIdentity::new(DEVICE_SECTORS).unwrap()
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
