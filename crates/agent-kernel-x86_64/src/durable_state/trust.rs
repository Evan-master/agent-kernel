//! Strict algorithm-bound verification against an isolated State Signer policy.
//!
//! This machine-layer policy borrows fixed-width Core records and owns no
//! mutable trust state. Identity, root scope, current generation, revocation,
//! and strict signature checks all complete before a verified value is issued.

use agent_kernel_core::{
    durable_state_signer_id_for_key, DurableArchiveManifest, DurableArchiveSignature,
    DurableStateDigest, DurableStatePublicKey, DurableStateSignerId, DurableStateSignerRecord,
    DurableStateSignerStatus,
};
use ed25519_dalek::{Signature as Ed25519Signature, VerifyingKey as Ed25519VerifyingKey};
use p256::ecdsa::{
    signature::Verifier, Signature as P256Signature, VerifyingKey as P256VerifyingKey,
};

use super::{durable_archive_manifest_digest, encode_durable_archive_manifest};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableStateVerificationError {
    SignerNotTrusted,
    TrustPolicyAmbiguous,
    SignerRevoked,
    SignerKeyIdMismatch,
    SignerKeyInvalid,
    SignerRootMismatch,
    PolicyGenerationMismatch,
    SignatureAlgorithmMismatch,
    SignatureNonCanonical,
    SignatureInvalid,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableStateTrustPolicy<'a> {
    signers: &'a [DurableStateSignerRecord],
    generation: u64,
}

impl<'a> DurableStateTrustPolicy<'a> {
    pub const fn new(signers: &'a [DurableStateSignerRecord], generation: u64) -> Self {
        Self {
            signers,
            generation,
        }
    }

    pub const fn signers(&self) -> &'a [DurableStateSignerRecord] {
        self.signers
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub fn verify(
        self,
        manifest: DurableArchiveManifest,
        signature: DurableArchiveSignature,
    ) -> Result<VerifiedDurableArchiveManifest, DurableStateVerificationError> {
        let mut matched = None;
        for signer in self.signers {
            if signer.signer_id == manifest.signer_id() {
                if matched.is_some() {
                    return Err(DurableStateVerificationError::TrustPolicyAmbiguous);
                }
                matched = Some(*signer);
            }
        }
        let signer = matched.ok_or(DurableStateVerificationError::SignerNotTrusted)?;
        if signer.status == DurableStateSignerStatus::Revoked {
            return Err(DurableStateVerificationError::SignerRevoked);
        }
        if durable_state_signer_id_for_key(signer.public_key) != signer.signer_id {
            return Err(DurableStateVerificationError::SignerKeyIdMismatch);
        }
        if signer.root != manifest.root() {
            return Err(DurableStateVerificationError::SignerRootMismatch);
        }
        if self.generation == 0
            || signer.generation != self.generation
            || manifest.signer_policy_generation() != self.generation
        {
            return Err(DurableStateVerificationError::PolicyGenerationMismatch);
        }
        if signer.signature_algorithm() != manifest.signature_algorithm() {
            return Err(DurableStateVerificationError::SignatureAlgorithmMismatch);
        }

        let signature_bytes = signature.bytes();
        let encoded = encode_durable_archive_manifest(manifest);
        match signer.public_key {
            DurableStatePublicKey::Ed25519(public_key) => {
                let signature = Ed25519Signature::from_bytes(&signature_bytes);
                let verifying_key = Ed25519VerifyingKey::from_bytes(&public_key)
                    .map_err(|_| DurableStateVerificationError::SignerKeyInvalid)?;
                verifying_key
                    .verify_strict(&encoded, &signature)
                    .map_err(|_| DurableStateVerificationError::SignatureInvalid)?;
            }
            DurableStatePublicKey::EcdsaP256(public_key) => {
                let signature = P256Signature::from_slice(&signature_bytes)
                    .map_err(|_| DurableStateVerificationError::SignatureInvalid)?;
                if signature.normalize_s().is_some() {
                    return Err(DurableStateVerificationError::SignatureNonCanonical);
                }
                let verifying_key = P256VerifyingKey::from_sec1_bytes(&public_key)
                    .map_err(|_| DurableStateVerificationError::SignerKeyInvalid)?;
                verifying_key
                    .verify(&encoded, &signature)
                    .map_err(|_| DurableStateVerificationError::SignatureInvalid)?;
            }
        }

        Ok(VerifiedDurableArchiveManifest {
            manifest,
            signer_id: signer.signer_id,
            manifest_digest: durable_archive_manifest_digest(manifest),
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VerifiedDurableArchiveManifest {
    manifest: DurableArchiveManifest,
    signer_id: DurableStateSignerId,
    manifest_digest: DurableStateDigest,
}

impl VerifiedDurableArchiveManifest {
    pub const fn manifest(self) -> DurableArchiveManifest {
        self.manifest
    }

    pub const fn signer_id(self) -> DurableStateSignerId {
        self.signer_id
    }

    pub const fn manifest_digest(self) -> DurableStateDigest {
        self.manifest_digest
    }
}
