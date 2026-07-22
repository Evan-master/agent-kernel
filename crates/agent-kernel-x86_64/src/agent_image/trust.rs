//! Strict Ed25519 verification over kernel-owned Trust Policy records.
//!
//! This machine-layer module borrows signer state from `agent-kernel-core` and
//! performs package-specific identity, scope, ABI, key, and signature checks.
//! It owns no mutable policy state and performs no allocation.

use agent_kernel_core::{
    agent_image_signer_id, AgentImageKind, AgentImageSignerId, AgentImageSignerRecord,
    AgentImageSignerStatus,
};
use ed25519_dalek::{Signature, VerifyingKey};

use super::{AgentImageCapsule, AgentImageLoadError, AGENT_PACKAGE_SIGNATURE_BYTES};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageTrustPolicy<'a> {
    signers: &'a [AgentImageSignerRecord],
}

impl<'a> AgentImageTrustPolicy<'a> {
    pub const fn new(signers: &'a [AgentImageSignerRecord]) -> Self {
        Self { signers }
    }

    pub const fn signers(&self) -> &'a [AgentImageSignerRecord] {
        self.signers
    }

    pub(super) fn verify(
        &self,
        capsule: &AgentImageCapsule<'_>,
        image_kind: AgentImageKind,
        abi_version: u16,
    ) -> Result<AgentImageSignerId, AgentImageLoadError> {
        let package_signer = capsule
            .signer_id()
            .ok_or(AgentImageLoadError::SignatureRequired)?;
        let mut matched = None;
        for signer in self.signers {
            if signer.signer_id == package_signer {
                if matched.is_some() {
                    return Err(AgentImageLoadError::TrustPolicyAmbiguous);
                }
                matched = Some(*signer);
            }
        }
        let signer = matched.ok_or(AgentImageLoadError::SignerNotTrusted)?;
        if signer.status == AgentImageSignerStatus::Revoked {
            return Err(AgentImageLoadError::SignerRevoked);
        }
        if agent_image_signer_id(signer.public_key) != package_signer {
            return Err(AgentImageLoadError::SignerKeyIdMismatch);
        }
        if !signer.image_kinds.allows(image_kind) {
            return Err(AgentImageLoadError::SignerScopeMismatch);
        }
        if abi_version < signer.minimum_abi || abi_version > signer.maximum_abi {
            return Err(AgentImageLoadError::SignerAbiMismatch);
        }

        let signature_bytes: &[u8; AGENT_PACKAGE_SIGNATURE_BYTES] = capsule
            .signature()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(AgentImageLoadError::InvalidSignatureLength)?;
        let signature = Signature::from_bytes(signature_bytes);
        let verifying_key = VerifyingKey::from_bytes(&signer.public_key)
            .map_err(|_| AgentImageLoadError::SignatureInvalid)?;
        verifying_key
            .verify_strict(
                capsule
                    .signed_bytes()
                    .ok_or(AgentImageLoadError::SignatureRequired)?,
                &signature,
            )
            .map_err(|_| AgentImageLoadError::SignatureInvalid)?;
        Ok(package_signer)
    }
}
