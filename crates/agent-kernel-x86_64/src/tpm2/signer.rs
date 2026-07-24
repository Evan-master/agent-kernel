//! Boot-bound TPM signer for durable archive manifests.
//!
//! This x86 component binds one persistent handle through ReadPublic, emits
//! fixed signing commands, verifies every result, and disables on runtime fault.

use agent_kernel_core::{
    durable_state_signer_id_for_key, DurableArchiveSignature, DurableSignatureAlgorithm,
    DurableStatePublicKey, DurableStateSignerId, DURABLE_ARCHIVE_MANIFEST_BYTES,
};
use agent_kernel_hal::TpmCommandTransport;
use p256::ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use super::{
    encode_read_public, encode_sign_p256_digest, parse_p256_signature_response,
    parse_read_public_response, public::verify_signing_public, DigestSignCommand,
    KernelStateSigner, KernelStateSignerError, TpmPersistentHandle, TpmPublicError, TpmWireError,
};

const TPM_ALG_SHA256: u16 = 0x000b;
const READ_PUBLIC_RESPONSE_BYTES: usize = 768;
const SIGN_RESPONSE_BYTES: usize = 128;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TpmSignerConfigError {
    ZeroPolicyGeneration,
    InvalidExpectedName,
    InvalidPublicKey,
}

impl<T: TpmCommandTransport> KernelStateSigner for ProvisionedTpmSigner<T> {
    fn signature_algorithm(&self) -> DurableSignatureAlgorithm {
        DurableSignatureAlgorithm::EcdsaP256Sha256
    }

    fn signer_id(&self) -> DurableStateSignerId {
        self.signer_id
    }

    fn policy_generation(&self) -> u64 {
        self.config.policy_generation
    }

    fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, KernelStateSignerError> {
        ProvisionedTpmSigner::sign_manifest(self, manifest)
            .map(DurableArchiveSignature::new)
            .map_err(|_| KernelStateSignerError::Unavailable)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ProvisionedTpmSignerConfig {
    handle: TpmPersistentHandle,
    mode: DigestSignCommand,
    policy_generation: u64,
    expected_name: [u8; 34],
    expected_public_key: [u8; 33],
}

impl ProvisionedTpmSignerConfig {
    pub fn new(
        handle: TpmPersistentHandle,
        mode: DigestSignCommand,
        policy_generation: u64,
        expected_name: [u8; 34],
        expected_public_key: [u8; 33],
    ) -> Result<Self, TpmSignerConfigError> {
        if policy_generation == 0 {
            return Err(TpmSignerConfigError::ZeroPolicyGeneration);
        }
        if expected_name[..2] != TPM_ALG_SHA256.to_be_bytes() {
            return Err(TpmSignerConfigError::InvalidExpectedName);
        }
        if VerifyingKey::from_sec1_bytes(&expected_public_key).is_err() {
            return Err(TpmSignerConfigError::InvalidPublicKey);
        }
        Ok(Self {
            handle,
            mode,
            policy_generation,
            expected_name,
            expected_public_key,
        })
    }

    pub const fn handle(self) -> TpmPersistentHandle {
        self.handle
    }

    pub const fn mode(self) -> DigestSignCommand {
        self.mode
    }

    pub const fn policy_generation(self) -> u64 {
        self.policy_generation
    }

    pub const fn expected_name(self) -> [u8; 34] {
        self.expected_name
    }

    pub const fn expected_public_key(self) -> [u8; 33] {
        self.expected_public_key
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TpmSignerError<E> {
    Transport(E),
    InvalidTransportLength { reported: usize, capacity: usize },
    Wire(TpmWireError),
    Public(TpmPublicError),
    SignatureVerification,
    Disabled,
}

pub struct ProvisionedTpmSigner<T> {
    transport: T,
    config: ProvisionedTpmSignerConfig,
    signer_id: DurableStateSignerId,
    disabled: bool,
}

impl<T: TpmCommandTransport> ProvisionedTpmSigner<T> {
    pub fn bind(
        mut transport: T,
        config: ProvisionedTpmSignerConfig,
    ) -> Result<Self, TpmSignerError<T::Error>> {
        let command = encode_read_public(config.handle);
        let mut response = [0; READ_PUBLIC_RESPONSE_BYTES];
        let length = transport
            .execute(&command, &mut response)
            .map_err(TpmSignerError::Transport)?;
        if length > response.len() {
            return Err(TpmSignerError::InvalidTransportLength {
                reported: length,
                capacity: response.len(),
            });
        }
        let decoded =
            parse_read_public_response(&response[..length]).map_err(TpmSignerError::Wire)?;
        verify_signing_public(&decoded, config.expected_name, config.expected_public_key)
            .map_err(TpmSignerError::Public)?;
        let public_key = DurableStatePublicKey::ecdsa_p256(config.expected_public_key)
            .ok_or(TpmSignerError::SignatureVerification)?;
        Ok(Self {
            transport,
            config,
            signer_id: durable_state_signer_id_for_key(public_key),
            disabled: false,
        })
    }

    pub const fn signer_id(&self) -> DurableStateSignerId {
        self.signer_id
    }

    pub const fn policy_generation(&self) -> u64 {
        self.config.policy_generation
    }

    pub const fn public_key(&self) -> [u8; 33] {
        self.config.expected_public_key
    }

    pub const fn mode(&self) -> DigestSignCommand {
        self.config.mode
    }

    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<[u8; 64], TpmSignerError<T::Error>> {
        if self.disabled {
            return Err(TpmSignerError::Disabled);
        }
        let digest: [u8; 32] = Sha256::digest(manifest).into();
        let command = encode_sign_p256_digest(self.config.handle, digest, self.config.mode);
        let mut response = [0; SIGN_RESPONSE_BYTES];
        let length = match self.transport.execute(&command, &mut response) {
            Ok(length) => length,
            Err(error) => {
                self.disabled = true;
                return Err(TpmSignerError::Transport(error));
            }
        };
        if length > response.len() {
            self.disabled = true;
            return Err(TpmSignerError::InvalidTransportLength {
                reported: length,
                capacity: response.len(),
            });
        }
        let encoded = match parse_p256_signature_response(&response[..length]) {
            Ok(signature) => signature,
            Err(error) => {
                self.disabled = true;
                return Err(TpmSignerError::Wire(error));
            }
        };
        if !self.signature_is_valid(digest, encoded) {
            self.disabled = true;
            return Err(TpmSignerError::SignatureVerification);
        }
        Ok(encoded)
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    fn signature_is_valid(&self, digest: [u8; 32], encoded: [u8; 64]) -> bool {
        let Ok(key) = VerifyingKey::from_sec1_bytes(&self.config.expected_public_key) else {
            return false;
        };
        let Ok(signature) = Signature::from_slice(&encoded) else {
            return false;
        };
        key.verify_prehash(&digest, &signature).is_ok()
    }
}
