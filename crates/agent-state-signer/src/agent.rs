//! State Signer policy enforcement and atomic request mutation.

use agent_kernel_core::DURABLE_ARCHIVE_SIGNATURE_BYTES;
use agent_kernel_x86_64::{
    durable_archive_request::{
        DurableArchiveRequest, DurableArchiveRequestDecodeError, DURABLE_ARCHIVE_REQUEST_BYTES,
        DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET,
    },
    durable_state::encode_durable_archive_manifest,
};

use crate::{StateSignerPolicy, StateSignerProvider};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateSignerAgentError<E> {
    Request(DurableArchiveRequestDecodeError),
    SignatureAlreadyPresent,
    SignerIdentityMismatch,
    RootMismatch,
    StorageMismatch,
    PolicyGenerationMismatch,
    EmptySignature,
    Provider(E),
}

pub struct StateSignerAgent<P> {
    policy: StateSignerPolicy,
    provider: P,
}

impl<P> StateSignerAgent<P>
where
    P: StateSignerProvider,
{
    pub const fn new(policy: StateSignerPolicy, provider: P) -> Self {
        Self { policy, provider }
    }

    pub const fn policy(&self) -> StateSignerPolicy {
        self.policy
    }

    pub const fn provider(&self) -> &P {
        &self.provider
    }

    pub fn sign_prepared_request(
        &mut self,
        bytes: &mut [u8; DURABLE_ARCHIVE_REQUEST_BYTES],
        expected_generation: u64,
    ) -> Result<DurableArchiveRequest, StateSignerAgentError<P::Error>> {
        let request = DurableArchiveRequest::decode(bytes, expected_generation)
            .map_err(StateSignerAgentError::Request)?;
        let signature_end =
            DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET + DURABLE_ARCHIVE_SIGNATURE_BYTES;
        if bytes[DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET..signature_end]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(StateSignerAgentError::SignatureAlreadyPresent);
        }

        let manifest = request.manifest();
        if self.provider.signer_id() != self.policy.signer_id()
            || manifest.signer_id() != self.policy.signer_id()
        {
            return Err(StateSignerAgentError::SignerIdentityMismatch);
        }
        if manifest.root() != self.policy.root() {
            return Err(StateSignerAgentError::RootMismatch);
        }
        if manifest.storage() != self.policy.storage() {
            return Err(StateSignerAgentError::StorageMismatch);
        }
        if manifest.signer_policy_generation() != self.policy.generation() {
            return Err(StateSignerAgentError::PolicyGenerationMismatch);
        }

        let encoded_manifest = encode_durable_archive_manifest(manifest);
        let signature = self
            .provider
            .sign_manifest(&encoded_manifest)
            .map_err(StateSignerAgentError::Provider)?;
        let signature = signature.bytes();
        if signature.iter().all(|byte| *byte == 0) {
            return Err(StateSignerAgentError::EmptySignature);
        }
        bytes[DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET..signature_end].copy_from_slice(&signature);

        DurableArchiveRequest::decode(bytes, expected_generation)
            .map_err(StateSignerAgentError::Request)
    }
}
