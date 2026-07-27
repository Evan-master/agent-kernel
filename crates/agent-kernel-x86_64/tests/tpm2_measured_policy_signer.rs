mod tpm2_support;

use agent_kernel_core::DURABLE_ARCHIVE_MANIFEST_BYTES;
use agent_kernel_x86_64::tpm2::{
    DigestSignCommand, ProvisionedTpmSigner, ProvisionedTpmSignerConfig, Sha256PcrPolicy,
    TpmPersistentHandle, TpmPolicySessionHandle, TpmPublicError, TpmSignerError, TpmWireError,
};
use p256::ecdsa::{
    signature::hazmat::{PrehashSigner, PrehashVerifier},
    Signature, SigningKey,
};
use sha2::{Digest, Sha256};

use tpm2_support::{
    command_error, command_success, policy_public_fixture, policy_session_response,
    policy_signature_response, ScriptedTpm,
};

const KEY_HANDLE: TpmPersistentHandle =
    TpmPersistentHandle::new(0x8101_0001).expect("persistent handle");
const SESSION_HANDLE: TpmPolicySessionHandle =
    TpmPolicySessionHandle::new(0x0300_0042).expect("policy session handle");
const POLICY_GENERATION: u64 = 24;
const POLICY_ATTRIBUTES: u32 = 0x0004_00b2;
const PCR_SELECTION: [u8; 3] = [0x81, 0x08, 0];
const PCR_DIGEST: [u8; 32] = [0xa5; 32];
const TPM_NONCE: [u8; 16] = [0xc3; 16];

#[test]
fn measured_policy_signer_runs_the_complete_session_lifecycle() {
    let key = SigningKey::from_slice(&[0x41; 32]).unwrap();
    let policy = pcr_policy();
    let mode = DigestSignCommand::SignDigestV185;
    let fixture = policy_public_fixture(&key, POLICY_ATTRIBUTES, policy.authorization_digest(mode));
    let manifest = [0x6a; DURABLE_ARCHIVE_MANIFEST_BYTES];
    let digest: [u8; 32] = Sha256::digest(manifest).into();
    let signature: Signature = key.sign_prehash(&digest).unwrap();
    let transport = ScriptedTpm::new([
        fixture.response.clone(),
        policy_session_response(SESSION_HANDLE, TPM_NONCE),
        command_success(),
        command_success(),
        policy_signature_response(signature, TPM_NONCE),
        command_success(),
    ]);
    let config = ProvisionedTpmSignerConfig::new_pcr_policy(
        KEY_HANDLE,
        mode,
        POLICY_GENERATION,
        fixture.name,
        fixture.compressed,
        policy,
    )
    .unwrap();

    let mut signer = ProvisionedTpmSigner::bind(transport, config).unwrap();
    let encoded = signer.sign_manifest(&manifest).unwrap();

    let parsed = Signature::from_slice(&encoded).unwrap();
    key.verifying_key()
        .verify_prehash(&digest, &parsed)
        .unwrap();
    assert!(!signer.is_disabled());

    let transport = signer.into_transport();
    let codes: Vec<u32> = transport
        .commands()
        .iter()
        .map(|command| u32::from_be_bytes(command[6..10].try_into().unwrap()))
        .collect();
    assert_eq!(codes, [0x173, 0x176, 0x17f, 0x16c, 0x1a6, 0x165]);
    assert_eq!(&transport.commands()[2][16..48], &PCR_DIGEST);
    assert_eq!(&transport.commands()[2][55..58], &PCR_SELECTION);
    assert_eq!(
        &transport.commands()[4][18..22],
        &SESSION_HANDLE.get().to_be_bytes()
    );
}

#[test]
fn measured_policy_binding_rejects_policy_mismatch_and_password_bypass() {
    let key = SigningKey::from_slice(&[0x42; 32]).unwrap();
    let policy = pcr_policy();
    let mode = DigestSignCommand::SignDigestV185;

    let wrong_policy = policy_public_fixture(&key, POLICY_ATTRIBUTES, [0x55; 32]);
    let config = policy_config(&wrong_policy, mode, policy);
    assert!(matches!(
        ProvisionedTpmSigner::bind(ScriptedTpm::new([wrong_policy.response]), config),
        Err(TpmSignerError::Public(
            TpmPublicError::AuthorizationPolicyMismatch
        ))
    ));

    let bypass_attributes = POLICY_ATTRIBUTES | (1 << 6);
    let bypass = policy_public_fixture(&key, bypass_attributes, policy.authorization_digest(mode));
    let config = policy_config(&bypass, mode, policy);
    assert!(matches!(
        ProvisionedTpmSigner::bind(ScriptedTpm::new([bypass.response]), config),
        Err(TpmSignerError::Public(
            TpmPublicError::ForbiddenAttributes { .. }
        ))
    ));
}

#[test]
fn pcr_rejection_flushes_the_session_and_disables_the_signer() {
    let key = SigningKey::from_slice(&[0x43; 32]).unwrap();
    let policy = pcr_policy();
    let mode = DigestSignCommand::SignDigestV185;
    let fixture = policy_public_fixture(&key, POLICY_ATTRIBUTES, policy.authorization_digest(mode));
    let transport = ScriptedTpm::new([
        fixture.response.clone(),
        policy_session_response(SESSION_HANDLE, TPM_NONCE),
        command_error(0x0000_0099),
        command_success(),
    ]);
    let config = policy_config(&fixture, mode, policy);
    let mut signer = ProvisionedTpmSigner::bind(transport, config).unwrap();

    assert_eq!(
        signer.sign_manifest(&[0x6b; DURABLE_ARCHIVE_MANIFEST_BYTES]),
        Err(TpmSignerError::Wire(TpmWireError::TpmResponseCode(
            0x0000_0099
        )))
    );
    assert!(signer.is_disabled());

    let transport = signer.into_transport();
    assert_eq!(
        u32::from_be_bytes(transport.commands()[3][6..10].try_into().unwrap()),
        0x165
    );
}

#[test]
fn cleanup_failure_discards_a_valid_signature_and_disables_the_signer() {
    let key = SigningKey::from_slice(&[0x44; 32]).unwrap();
    let policy = pcr_policy();
    let mode = DigestSignCommand::SignDigestV185;
    let fixture = policy_public_fixture(&key, POLICY_ATTRIBUTES, policy.authorization_digest(mode));
    let manifest = [0x6c; DURABLE_ARCHIVE_MANIFEST_BYTES];
    let digest: [u8; 32] = Sha256::digest(manifest).into();
    let signature: Signature = key.sign_prehash(&digest).unwrap();
    let transport = ScriptedTpm::new([
        fixture.response.clone(),
        policy_session_response(SESSION_HANDLE, TPM_NONCE),
        command_success(),
        command_success(),
        policy_signature_response(signature, TPM_NONCE),
        command_error(0x0000_018b),
    ]);
    let config = policy_config(&fixture, mode, policy);
    let mut signer = ProvisionedTpmSigner::bind(transport, config).unwrap();

    assert_eq!(
        signer.sign_manifest(&manifest),
        Err(TpmSignerError::SessionCleanup)
    );
    assert!(signer.is_disabled());
}

#[test]
fn malformed_start_response_recovers_and_flushes_its_session_handle() {
    let key = SigningKey::from_slice(&[0x45; 32]).unwrap();
    let policy = pcr_policy();
    let mode = DigestSignCommand::SignDigestV185;
    let fixture = policy_public_fixture(&key, POLICY_ATTRIBUTES, policy.authorization_digest(mode));
    let mut malformed = policy_session_response(SESSION_HANDLE, TPM_NONCE);
    malformed.pop();
    malformed[2..6].copy_from_slice(&31_u32.to_be_bytes());
    malformed[14..16].copy_from_slice(&15_u16.to_be_bytes());
    let transport = ScriptedTpm::new([fixture.response.clone(), malformed, command_success()]);
    let config = policy_config(&fixture, mode, policy);
    let mut signer = ProvisionedTpmSigner::bind(transport, config).unwrap();

    assert_eq!(
        signer.sign_manifest(&[0x6d; DURABLE_ARCHIVE_MANIFEST_BYTES]),
        Err(TpmSignerError::Wire(
            TpmWireError::InvalidPolicySessionNonce { declared: 15 }
        ))
    );
    assert!(signer.is_disabled());
    let transport = signer.into_transport();
    assert_eq!(
        u32::from_be_bytes(transport.commands()[2][6..10].try_into().unwrap()),
        0x165
    );
}

fn pcr_policy() -> Sha256PcrPolicy {
    Sha256PcrPolicy::new(PCR_SELECTION, PCR_DIGEST).unwrap()
}

fn policy_config(
    fixture: &tpm2_support::PublicFixture,
    mode: DigestSignCommand,
    policy: Sha256PcrPolicy,
) -> ProvisionedTpmSignerConfig {
    ProvisionedTpmSignerConfig::new_pcr_policy(
        KEY_HANDLE,
        mode,
        POLICY_GENERATION,
        fixture.name,
        fixture.compressed,
        policy,
    )
    .unwrap()
}
