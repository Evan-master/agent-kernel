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

use super::wire::recover_start_policy_session_handle;
use super::{
    encode_flush_context, encode_policy_command_code, encode_policy_pcr,
    encode_policy_sign_p256_digest, encode_read_public, encode_sign_p256_digest,
    encode_start_policy_session, parse_command_success, parse_p256_policy_signature_response,
    parse_p256_signature_response, parse_read_public_response, parse_start_policy_session_response,
    public::{verify_signing_public, ExpectedPublicAuthorization},
    DigestSignCommand, KernelStateSigner, KernelStateSignerError, Sha256PcrPolicy,
    TpmPersistentHandle, TpmPolicySessionHandle, TpmPublicError, TpmWireError,
};

const TPM_ALG_SHA256: u16 = 0x000b;
const READ_PUBLIC_RESPONSE_BYTES: usize = 768;
const SIGN_RESPONSE_BYTES: usize = 128;
const POLICY_SESSION_RESPONSE_BYTES: usize = 64;
const COMMAND_RESPONSE_BYTES: usize = 10;

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
            .map_err(|error| match error {
                TpmSignerError::Wire(TpmWireError::TpmResponseCode(code)) => {
                    KernelStateSignerError::TpmResponseCode(code)
                }
                _ => KernelStateSignerError::Unavailable,
            })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TpmSignerAuthorization {
    EmptyPassword,
    PcrPolicy(Sha256PcrPolicy),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ProvisionedTpmSignerConfig {
    handle: TpmPersistentHandle,
    mode: DigestSignCommand,
    policy_generation: u64,
    expected_name: [u8; 34],
    expected_public_key: [u8; 33],
    authorization: TpmSignerAuthorization,
}

impl ProvisionedTpmSignerConfig {
    pub fn new(
        handle: TpmPersistentHandle,
        mode: DigestSignCommand,
        policy_generation: u64,
        expected_name: [u8; 34],
        expected_public_key: [u8; 33],
    ) -> Result<Self, TpmSignerConfigError> {
        Self::new_with_authorization(
            handle,
            mode,
            policy_generation,
            expected_name,
            expected_public_key,
            TpmSignerAuthorization::EmptyPassword,
        )
    }

    pub fn new_pcr_policy(
        handle: TpmPersistentHandle,
        mode: DigestSignCommand,
        policy_generation: u64,
        expected_name: [u8; 34],
        expected_public_key: [u8; 33],
        policy: Sha256PcrPolicy,
    ) -> Result<Self, TpmSignerConfigError> {
        Self::new_with_authorization(
            handle,
            mode,
            policy_generation,
            expected_name,
            expected_public_key,
            TpmSignerAuthorization::PcrPolicy(policy),
        )
    }

    fn new_with_authorization(
        handle: TpmPersistentHandle,
        mode: DigestSignCommand,
        policy_generation: u64,
        expected_name: [u8; 34],
        expected_public_key: [u8; 33],
        authorization: TpmSignerAuthorization,
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
            authorization,
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

    pub const fn authorization(self) -> TpmSignerAuthorization {
        self.authorization
    }

    fn expected_public_authorization(self) -> ExpectedPublicAuthorization {
        match self.authorization {
            TpmSignerAuthorization::EmptyPassword => ExpectedPublicAuthorization::EmptyPassword,
            TpmSignerAuthorization::PcrPolicy(policy) => {
                ExpectedPublicAuthorization::PcrPolicy(policy.authorization_digest(self.mode))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TpmSignerError<E> {
    Transport(E),
    InvalidTransportLength { reported: usize, capacity: usize },
    Wire(TpmWireError),
    Public(TpmPublicError),
    SignatureVerification,
    SessionCleanup,
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
        verify_signing_public(
            &decoded,
            config.expected_name,
            config.expected_public_key,
            config.expected_public_authorization(),
        )
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
        let result = match self.config.authorization {
            TpmSignerAuthorization::EmptyPassword => self.sign_with_password(digest),
            TpmSignerAuthorization::PcrPolicy(policy) => self.sign_with_pcr_policy(digest, policy),
        };
        if result.is_err() {
            self.disabled = true;
        }
        result
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    fn sign_with_password(
        &mut self,
        digest: [u8; 32],
    ) -> Result<[u8; 64], TpmSignerError<T::Error>> {
        let command = encode_sign_p256_digest(self.config.handle, digest, self.config.mode);
        let mut response = [0; SIGN_RESPONSE_BYTES];
        let length = self.execute_command(&command, &mut response)?;
        let encoded =
            parse_p256_signature_response(&response[..length]).map_err(TpmSignerError::Wire)?;
        if !self.signature_is_valid(digest, encoded) {
            return Err(TpmSignerError::SignatureVerification);
        }
        Ok(encoded)
    }

    fn sign_with_pcr_policy(
        &mut self,
        digest: [u8; 32],
        policy: Sha256PcrPolicy,
    ) -> Result<[u8; 64], TpmSignerError<T::Error>> {
        let mut nonce = [0; 16];
        nonce.copy_from_slice(&digest[..16]);
        let command = encode_start_policy_session(nonce);
        let mut response = [0; POLICY_SESSION_RESPONSE_BYTES];
        let length = self.execute_command(&command, &mut response)?;
        let session = match parse_start_policy_session_response(&response[..length]) {
            Ok(session) => session,
            Err(error) => {
                if let Some(session) = recover_start_policy_session_handle(&response[..length]) {
                    if self.flush_policy_session(session).is_err() {
                        return Err(TpmSignerError::SessionCleanup);
                    }
                }
                return Err(TpmSignerError::Wire(error));
            }
        };

        let operation = self.run_policy_signature(session, digest, policy);
        if self.flush_policy_session(session).is_err() {
            return Err(TpmSignerError::SessionCleanup);
        }
        let encoded = operation?;
        if !self.signature_is_valid(digest, encoded) {
            return Err(TpmSignerError::SignatureVerification);
        }
        Ok(encoded)
    }

    fn run_policy_signature(
        &mut self,
        session: TpmPolicySessionHandle,
        digest: [u8; 32],
        policy: Sha256PcrPolicy,
    ) -> Result<[u8; 64], TpmSignerError<T::Error>> {
        self.execute_success(&encode_policy_pcr(session, policy))?;
        self.execute_success(&encode_policy_command_code(session, self.config.mode))?;
        let command =
            encode_policy_sign_p256_digest(self.config.handle, digest, self.config.mode, session);
        let mut response = [0; SIGN_RESPONSE_BYTES];
        let length = self.execute_command(&command, &mut response)?;
        parse_p256_policy_signature_response(&response[..length]).map_err(TpmSignerError::Wire)
    }

    fn flush_policy_session(
        &mut self,
        session: TpmPolicySessionHandle,
    ) -> Result<(), TpmSignerError<T::Error>> {
        self.execute_success(&encode_flush_context(session))
    }

    fn execute_success(&mut self, command: &[u8]) -> Result<(), TpmSignerError<T::Error>> {
        let mut response = [0; COMMAND_RESPONSE_BYTES];
        let length = self.execute_command(command, &mut response)?;
        parse_command_success(&response[..length]).map_err(TpmSignerError::Wire)
    }

    fn execute_command(
        &mut self,
        command: &[u8],
        response: &mut [u8],
    ) -> Result<usize, TpmSignerError<T::Error>> {
        let length = self
            .transport
            .execute(command, response)
            .map_err(TpmSignerError::Transport)?;
        if length > response.len() {
            return Err(TpmSignerError::InvalidTransportLength {
                reported: length,
                capacity: response.len(),
            });
        }
        Ok(length)
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
