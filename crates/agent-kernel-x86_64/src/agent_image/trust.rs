//! Immutable boot trust policy for signed Agent Packages.

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use super::{AgentImageCapsule, AgentImageLoadError, AGENT_PACKAGE_SIGNATURE_BYTES};

const SIGNER_ID_DOMAIN: &[u8] = b"AGENT_KERNEL_ED25519_SIGNER_V1\0";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageSignerId {
    bytes: [u8; 32],
}

impl AgentImageSignerId {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.bytes
    }

    pub const fn is_zero(self) -> bool {
        let mut index = 0;
        while index < self.bytes.len() {
            if self.bytes[index] != 0 {
                return false;
            }
            index += 1;
        }
        true
    }
}

pub fn agent_image_signer_id(public_key: [u8; 32]) -> AgentImageSignerId {
    let mut digest = Sha256::new();
    digest.update(SIGNER_ID_DOMAIN);
    digest.update(public_key);
    AgentImageSignerId::new(digest.finalize().into())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageKindScope {
    bits: u8,
}

impl AgentImageKindScope {
    pub const fn only(image_kind: u16) -> Option<Self> {
        let bits = match image_kind {
            1 => 1,
            2 => 2,
            3 => 4,
            4 => 8,
            _ => return None,
        };
        Some(Self { bits })
    }

    pub const fn all() -> Self {
        Self { bits: 0b1111 }
    }

    pub const fn allows(self, image_kind: u16) -> bool {
        match image_kind {
            1 => self.bits & 1 != 0,
            2 => self.bits & 2 != 0,
            3 => self.bits & 4 != 0,
            4 => self.bits & 8 != 0,
            _ => false,
        }
    }

    const fn is_empty(self) -> bool {
        self.bits == 0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TrustedSignerStatus {
    Active,
    Revoked,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TrustedAgentSigner {
    signer_id: AgentImageSignerId,
    public_key: [u8; 32],
    image_kinds: AgentImageKindScope,
    minimum_abi: u16,
    maximum_abi: u16,
    status: TrustedSignerStatus,
}

impl TrustedAgentSigner {
    pub const fn new(
        signer_id: AgentImageSignerId,
        public_key: [u8; 32],
        image_kinds: AgentImageKindScope,
        minimum_abi: u16,
        maximum_abi: u16,
        status: TrustedSignerStatus,
    ) -> Option<Self> {
        if signer_id.is_zero()
            || image_kinds.is_empty()
            || minimum_abi == 0
            || minimum_abi > maximum_abi
        {
            return None;
        }
        Some(Self {
            signer_id,
            public_key,
            image_kinds,
            minimum_abi,
            maximum_abi,
            status,
        })
    }

    pub const fn signer_id(self) -> AgentImageSignerId {
        self.signer_id
    }

    pub const fn public_key(self) -> [u8; 32] {
        self.public_key
    }

    pub const fn status(self) -> TrustedSignerStatus {
        self.status
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageTrustPolicy<const N: usize> {
    signers: [TrustedAgentSigner; N],
}

impl<const N: usize> AgentImageTrustPolicy<N> {
    pub const fn new(signers: [TrustedAgentSigner; N]) -> Self {
        Self { signers }
    }

    pub const fn signers(&self) -> &[TrustedAgentSigner; N] {
        &self.signers
    }

    pub(super) fn verify(
        &self,
        capsule: &AgentImageCapsule<'_>,
    ) -> Result<AgentImageSignerId, AgentImageLoadError> {
        let package_signer = capsule
            .signer_id()
            .ok_or(AgentImageLoadError::SignatureRequired)?;
        let mut matched = None;
        for signer in &self.signers {
            if signer.signer_id == package_signer {
                if matched.is_some() {
                    return Err(AgentImageLoadError::TrustPolicyAmbiguous);
                }
                matched = Some(*signer);
            }
        }
        let signer = matched.ok_or(AgentImageLoadError::SignerNotTrusted)?;
        if signer.status == TrustedSignerStatus::Revoked {
            return Err(AgentImageLoadError::SignerRevoked);
        }
        if agent_image_signer_id(signer.public_key) != package_signer {
            return Err(AgentImageLoadError::SignerKeyIdMismatch);
        }
        let header = capsule.header();
        if !signer.image_kinds.allows(header.image_kind()) {
            return Err(AgentImageLoadError::SignerScopeMismatch);
        }
        if header.abi_version() < signer.minimum_abi || header.abi_version() > signer.maximum_abi {
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
