mod tpm2_support;

use agent_kernel_core::{
    durable_state_signer_id_for_key, DurableStatePublicKey, DURABLE_ARCHIVE_MANIFEST_BYTES,
};
use agent_kernel_hal::TpmCommandTransport;
use agent_kernel_x86_64::tpm2::{
    DigestSignCommand, ProvisionedTpmSigner, ProvisionedTpmSignerConfig, TpmPersistentHandle,
    TpmPublicError, TpmSignerError, TpmWireError,
};
use p256::ecdsa::{
    signature::hazmat::{PrehashSigner, PrehashVerifier},
    Signature, SigningKey,
};
use sha2::{Digest, Sha256};

use tpm2_support::{public_fixture, signature_response, ScriptedTpm, TransportError};

const HANDLE: TpmPersistentHandle =
    TpmPersistentHandle::new(0x8101_0001).expect("persistent handle");
const POLICY_GENERATION: u64 = 23;

struct OverreportingTpm {
    inner: ScriptedTpm,
    overreport_after: usize,
    calls: usize,
}

impl TpmCommandTransport for OverreportingTpm {
    type Error = TransportError;

    fn execute(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, Self::Error> {
        let length = self.inner.execute(command, response)?;
        self.calls += 1;
        Ok(if self.calls > self.overreport_after {
            response.len() + 1
        } else {
            length
        })
    }
}

#[test]
fn signer_binds_read_public_and_signs_the_exact_manifest_digest() {
    let key = SigningKey::from_slice(&[0x37; 32]).unwrap();
    let fixture = public_fixture(&key, 0x0004_0072);
    let manifest = [0xa5; DURABLE_ARCHIVE_MANIFEST_BYTES];
    let digest: [u8; 32] = Sha256::digest(manifest).into();
    let signature: Signature = key.sign_prehash(&digest).unwrap();
    let transport = ScriptedTpm::new([fixture.response.clone(), signature_response(signature)]);
    let config = ProvisionedTpmSignerConfig::new(
        HANDLE,
        DigestSignCommand::SignDigestV185,
        POLICY_GENERATION,
        fixture.name,
        fixture.compressed,
    )
    .unwrap();

    let mut signer = ProvisionedTpmSigner::bind(transport, config).unwrap();
    let encoded = signer.sign_manifest(&manifest).unwrap();

    let expected_id = durable_state_signer_id_for_key(
        DurableStatePublicKey::ecdsa_p256(fixture.compressed).unwrap(),
    );
    assert_eq!(signer.signer_id(), expected_id);
    assert_eq!(signer.policy_generation(), POLICY_GENERATION);
    assert_eq!(signer.public_key(), fixture.compressed);
    let parsed = Signature::from_slice(&encoded).unwrap();
    key.verifying_key()
        .verify_prehash(&digest, &parsed)
        .unwrap();

    let transport = signer.into_transport();
    assert_eq!(
        &transport.commands()[0][6..10],
        &0x0000_0173_u32.to_be_bytes()
    );
    assert_eq!(
        &transport.commands()[1][6..10],
        &0x0000_01a6_u32.to_be_bytes()
    );
    assert_eq!(&transport.commands()[1][31..63], &digest);
}

#[test]
fn binding_rejects_name_and_template_policy_changes() {
    let key = SigningKey::from_slice(&[0x38; 32]).unwrap();
    let fixture = public_fixture(&key, 0x0004_0072);
    let mut wrong_name = fixture.name;
    wrong_name[33] ^= 1;
    let config = ProvisionedTpmSignerConfig::new(
        HANDLE,
        DigestSignCommand::SignV184,
        POLICY_GENERATION,
        wrong_name,
        fixture.compressed,
    )
    .unwrap();
    assert!(matches!(
        ProvisionedTpmSigner::bind(ScriptedTpm::new([fixture.response]), config),
        Err(TpmSignerError::Public(TpmPublicError::NameMismatch))
    ));

    let restricted = public_fixture(&key, 0x0005_0072);
    let config = ProvisionedTpmSignerConfig::new(
        HANDLE,
        DigestSignCommand::SignV184,
        POLICY_GENERATION,
        restricted.name,
        restricted.compressed,
    )
    .unwrap();
    assert!(matches!(
        ProvisionedTpmSigner::bind(ScriptedTpm::new([restricted.response]), config),
        Err(TpmSignerError::Public(
            TpmPublicError::ForbiddenAttributes { .. }
        ))
    ));

    let mut invalid_point = public_fixture(&key, 0x0004_0072);
    invalid_point.response[68] ^= 1;
    let public_digest: [u8; 32] = Sha256::digest(&invalid_point.response[12..100]).into();
    invalid_point.name[2..].copy_from_slice(&public_digest);
    invalid_point.response[104..136].copy_from_slice(&public_digest);
    let config = ProvisionedTpmSignerConfig::new(
        HANDLE,
        DigestSignCommand::SignV184,
        POLICY_GENERATION,
        invalid_point.name,
        invalid_point.compressed,
    )
    .unwrap();
    let result = ProvisionedTpmSigner::bind(ScriptedTpm::new([invalid_point.response]), config);
    let error = match result {
        Ok(_) => panic!("invalid TPM point was accepted"),
        Err(error) => error,
    };
    assert_eq!(error, TpmSignerError::Public(TpmPublicError::InvalidPoint));
}

#[test]
fn a_runtime_tpm_failure_disables_the_signer_for_the_boot() {
    let key = SigningKey::from_slice(&[0x39; 32]).unwrap();
    let fixture = public_fixture(&key, 0x0004_0072);
    let config = ProvisionedTpmSignerConfig::new(
        HANDLE,
        DigestSignCommand::SignDigestV185,
        POLICY_GENERATION,
        fixture.name,
        fixture.compressed,
    )
    .unwrap();
    let error_response = vec![0x80, 0x01, 0, 0, 0, 10, 0, 0, 1, 0x01];
    let mut signer =
        ProvisionedTpmSigner::bind(ScriptedTpm::new([fixture.response, error_response]), config)
            .unwrap();
    let manifest = [0x5c; DURABLE_ARCHIVE_MANIFEST_BYTES];

    assert_eq!(
        signer.sign_manifest(&manifest),
        Err(TpmSignerError::Wire(TpmWireError::TpmResponseCode(0x101)))
    );
    assert_eq!(
        signer.sign_manifest(&manifest),
        Err(TpmSignerError::Disabled)
    );
}

#[test]
fn transport_lengths_cannot_escape_kernel_owned_response_buffers() {
    let key = SigningKey::from_slice(&[0x3a; 32]).unwrap();
    let fixture = public_fixture(&key, 0x0004_0072);
    let config = ProvisionedTpmSignerConfig::new(
        HANDLE,
        DigestSignCommand::SignDigestV185,
        POLICY_GENERATION,
        fixture.name,
        fixture.compressed,
    )
    .unwrap();
    let bind_transport = OverreportingTpm {
        inner: ScriptedTpm::new([fixture.response.clone()]),
        overreport_after: 0,
        calls: 0,
    };
    let result = ProvisionedTpmSigner::bind(bind_transport, config);
    assert!(matches!(
        result,
        Err(TpmSignerError::InvalidTransportLength {
            reported: 769,
            capacity: 768
        })
    ));

    let digest: [u8; 32] = Sha256::digest([0x5d; DURABLE_ARCHIVE_MANIFEST_BYTES]).into();
    let signature: Signature = key.sign_prehash(&digest).unwrap();
    let runtime_transport = OverreportingTpm {
        inner: ScriptedTpm::new([fixture.response, signature_response(signature)]),
        overreport_after: 1,
        calls: 0,
    };
    let mut signer = ProvisionedTpmSigner::bind(runtime_transport, config).unwrap();
    let manifest = [0x5d; DURABLE_ARCHIVE_MANIFEST_BYTES];
    assert_eq!(
        signer.sign_manifest(&manifest),
        Err(TpmSignerError::InvalidTransportLength {
            reported: 129,
            capacity: 128
        })
    );
    assert!(signer.is_disabled());
}
