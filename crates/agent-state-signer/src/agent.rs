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
    SignatureAlgorithmMismatch,
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
        if self.provider.signature_algorithm() != self.policy.signature_algorithm()
            || manifest.signature_algorithm() != self.policy.signature_algorithm()
        {
            return Err(StateSignerAgentError::SignatureAlgorithmMismatch);
        }
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
        let mut signature = signature.bytes();
        if signature.iter().all(|byte| *byte == 0) {
            return Err(StateSignerAgentError::EmptySignature);
        }
        if self.policy.signature_algorithm()
            == agent_kernel_core::DurableSignatureAlgorithm::EcdsaP256Sha256
        {
            normalize_p256_s(&mut signature);
        }
        bytes[DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET..signature_end].copy_from_slice(&signature);

        DurableArchiveRequest::decode(bytes, expected_generation)
            .map_err(StateSignerAgentError::Request)
    }
}

const P256_ORDER: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2, 0xfc, 0x63, 0x25, 0x51,
];
const P256_HALF_ORDER: [u8; 32] = [
    0x7f, 0xff, 0xff, 0xff, 0x80, 0x00, 0x00, 0x00, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xde, 0x73, 0x7d, 0x56, 0xd3, 0x8b, 0xcf, 0x42, 0x79, 0xdc, 0xe5, 0x61, 0x7e, 0x31, 0x92, 0xa8,
];

fn normalize_p256_s(signature: &mut [u8; DURABLE_ARCHIVE_SIGNATURE_BYTES]) {
    let mut is_high = false;
    for (actual, half) in signature[32..].iter().zip(P256_HALF_ORDER) {
        if *actual > half {
            is_high = true;
            break;
        }
        if *actual < half {
            break;
        }
    }
    if !is_high {
        return;
    }

    let mut borrow = 0u16;
    for index in (0..32).rev() {
        let minuend = u16::from(P256_ORDER[index]);
        let subtrahend = u16::from(signature[32 + index]) + borrow;
        if minuend >= subtrahend {
            signature[32 + index] = (minuend - subtrahend) as u8;
            borrow = 0;
        } else {
            signature[32 + index] = (minuend + 256 - subtrahend) as u8;
            borrow = 1;
        }
    }
}
