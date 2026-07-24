//! Kernel-only durable archive signing service.
//!
//! This x86 service exposes a narrow signer trait and signs only an unchanged,
//! retained durable Manifest after generation, identity, and algorithm checks.

use agent_kernel_core::{
    DurableArchiveManifest, DurableArchiveSignature, DurableSignatureAlgorithm,
    DurableStateSignerId, DURABLE_ARCHIVE_MANIFEST_BYTES,
};

use crate::{
    durable_archive_request::{
        DurableArchiveRequest, DurableArchiveRequestDecodeError, DURABLE_ARCHIVE_REQUEST_BYTES,
        DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET, DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET,
    },
    durable_state::encode_durable_archive_manifest,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum KernelStateSignerError {
    Unavailable,
}

pub trait KernelStateSigner {
    fn signature_algorithm(&self) -> DurableSignatureAlgorithm;
    fn signer_id(&self) -> DurableStateSignerId;
    fn policy_generation(&self) -> u64;

    fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, KernelStateSignerError>;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum KernelStateSignerServiceError {
    RequestChanged,
    Request(DurableArchiveRequestDecodeError),
    SignatureAlreadyPresent,
    ManifestChanged,
    SignatureAlgorithmMismatch,
    SignerIdentityMismatch,
    PolicyGenerationMismatch,
    Signer(KernelStateSignerError),
    EmptySignature,
    SignedRequestInvalid,
}

pub fn sign_retained_durable_request(
    retained_request: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
    current_request: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
    retained_manifest: DurableArchiveManifest,
    expected_generation: u64,
    signer: &mut dyn KernelStateSigner,
) -> Result<[u8; DURABLE_ARCHIVE_REQUEST_BYTES], KernelStateSignerServiceError> {
    if retained_request != current_request {
        return Err(KernelStateSignerServiceError::RequestChanged);
    }
    let request = DurableArchiveRequest::decode(current_request, expected_generation)
        .map_err(KernelStateSignerServiceError::Request)?;
    if current_request
        [DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET..DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(KernelStateSignerServiceError::SignatureAlreadyPresent);
    }
    let manifest = request.manifest();
    if manifest != retained_manifest {
        return Err(KernelStateSignerServiceError::ManifestChanged);
    }
    if manifest.signature_algorithm() != DurableSignatureAlgorithm::EcdsaP256Sha256
        || signer.signature_algorithm() != DurableSignatureAlgorithm::EcdsaP256Sha256
    {
        return Err(KernelStateSignerServiceError::SignatureAlgorithmMismatch);
    }
    if manifest.signer_id() != signer.signer_id() {
        return Err(KernelStateSignerServiceError::SignerIdentityMismatch);
    }
    if manifest.signer_policy_generation() != signer.policy_generation() {
        return Err(KernelStateSignerServiceError::PolicyGenerationMismatch);
    }

    let encoded_manifest = encode_durable_archive_manifest(manifest);
    let signature = signer
        .sign_manifest(&encoded_manifest)
        .map_err(KernelStateSignerServiceError::Signer)?;
    let signature = signature.bytes();
    if signature.iter().all(|byte| *byte == 0) {
        return Err(KernelStateSignerServiceError::EmptySignature);
    }
    let mut signed = *current_request;
    signed[DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET..DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET]
        .copy_from_slice(&signature);
    let decoded = DurableArchiveRequest::decode(&signed, expected_generation)
        .map_err(|_| KernelStateSignerServiceError::SignedRequestInvalid)?;
    if decoded.manifest() != retained_manifest || decoded.signature().bytes() != signature {
        return Err(KernelStateSignerServiceError::SignedRequestInvalid);
    }
    Ok(signed)
}
