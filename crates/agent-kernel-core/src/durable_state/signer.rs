//! State Signer identity and policy-record values.
//!
//! The Core layer derives a domain-separated identity and records root scope,
//! status, and policy generation. Ed25519 key parsing and verification remain
//! machine-verifier responsibilities.

use sha2::{Digest, Sha256};

use crate::ResourceId;

const SIGNER_ID_DOMAIN: &[u8] = b"AGENT-KERNEL-DURABLE-STATE-SIGNER-V1\0";

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
    digest.update(SIGNER_ID_DOMAIN);
    digest.update(public_key);
    DurableStateSignerId::new(digest.finalize().into())
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
    pub public_key: [u8; 32],
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
        if root.raw() == 0 || generation == 0 {
            return None;
        }
        Some(Self {
            signer_id: durable_state_signer_id(public_key),
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
}
