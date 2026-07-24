//! State Signer identity and policy-record values.
//!
//! The Core layer derives a domain-separated identity and records root scope,
//! status, and policy generation. Algorithm-specific key parsing and signature
//! verification remain machine-verifier responsibilities.

use sha2::{Digest, Sha256};

use crate::ResourceId;

const LEGACY_SIGNER_ID_DOMAIN: &[u8] = b"AGENT-KERNEL-DURABLE-STATE-SIGNER-V1\0";
const ALGORITHM_SIGNER_ID_DOMAIN: &[u8] = b"AGENT-KERNEL-DURABLE-STATE-SIGNER-V2\0";

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum DurableSignatureAlgorithm {
    Ed25519 = 1,
    EcdsaP256Sha256 = 2,
}

impl DurableSignatureAlgorithm {
    pub const fn wire_value(self) -> u16 {
        self as u16
    }

    pub const fn from_wire_value(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::Ed25519),
            2 => Some(Self::EcdsaP256Sha256),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableStatePublicKey {
    Ed25519([u8; 32]),
    EcdsaP256([u8; 33]),
}

impl DurableStatePublicKey {
    pub const fn ed25519(bytes: [u8; 32]) -> Self {
        Self::Ed25519(bytes)
    }

    pub const fn ecdsa_p256(bytes: [u8; 33]) -> Option<Self> {
        if bytes[0] == 0x02 || bytes[0] == 0x03 {
            Some(Self::EcdsaP256(bytes))
        } else {
            None
        }
    }

    pub const fn algorithm(self) -> DurableSignatureAlgorithm {
        match self {
            Self::Ed25519(_) => DurableSignatureAlgorithm::Ed25519,
            Self::EcdsaP256(_) => DurableSignatureAlgorithm::EcdsaP256Sha256,
        }
    }

    pub const fn ed25519_bytes(self) -> Option<[u8; 32]> {
        match self {
            Self::Ed25519(bytes) => Some(bytes),
            Self::EcdsaP256(_) => None,
        }
    }

    pub const fn ecdsa_p256_bytes(self) -> Option<[u8; 33]> {
        match self {
            Self::Ed25519(_) => None,
            Self::EcdsaP256(bytes) => Some(bytes),
        }
    }

    pub const fn has_canonical_encoding(self) -> bool {
        match self {
            Self::Ed25519(_) => true,
            Self::EcdsaP256(bytes) => bytes[0] == 0x02 || bytes[0] == 0x03,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableStateSignerId {
    bytes: [u8; 32],
}

impl DurableStateSignerId {
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

pub fn durable_state_signer_id(public_key: [u8; 32]) -> DurableStateSignerId {
    let mut digest = Sha256::new();
    digest.update(LEGACY_SIGNER_ID_DOMAIN);
    digest.update(public_key);
    DurableStateSignerId::new(digest.finalize().into())
}

pub fn durable_state_signer_id_for_key(public_key: DurableStatePublicKey) -> DurableStateSignerId {
    match public_key {
        DurableStatePublicKey::Ed25519(bytes) => durable_state_signer_id(bytes),
        DurableStatePublicKey::EcdsaP256(bytes) => {
            let mut digest = Sha256::new();
            digest.update(ALGORITHM_SIGNER_ID_DOMAIN);
            digest.update(
                DurableSignatureAlgorithm::EcdsaP256Sha256
                    .wire_value()
                    .to_le_bytes(),
            );
            digest.update(bytes);
            DurableStateSignerId::new(digest.finalize().into())
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DurableStateSignerStatus {
    Active = 1,
    Revoked = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableStateSignerRecord {
    pub signer_id: DurableStateSignerId,
    pub root: ResourceId,
    pub public_key: DurableStatePublicKey,
    pub status: DurableStateSignerStatus,
    pub generation: u64,
}

impl DurableStateSignerRecord {
    pub fn new(
        root: ResourceId,
        public_key: [u8; 32],
        status: DurableStateSignerStatus,
        generation: u64,
    ) -> Option<Self> {
        Self::new_with_key(
            root,
            DurableStatePublicKey::ed25519(public_key),
            status,
            generation,
        )
    }

    pub fn new_with_key(
        root: ResourceId,
        public_key: DurableStatePublicKey,
        status: DurableStateSignerStatus,
        generation: u64,
    ) -> Option<Self> {
        if root.raw() == 0 || generation == 0 || !public_key.has_canonical_encoding() {
            return None;
        }
        Some(Self {
            signer_id: durable_state_signer_id_for_key(public_key),
            root,
            public_key,
            status,
            generation,
        })
    }

    pub const fn allows(self, root: ResourceId, policy_generation: u64) -> bool {
        self.status as u8 == DurableStateSignerStatus::Active as u8
            && self.root.raw() == root.raw()
            && self.generation == policy_generation
    }

    pub const fn signature_algorithm(self) -> DurableSignatureAlgorithm {
        self.public_key.algorithm()
    }
}
