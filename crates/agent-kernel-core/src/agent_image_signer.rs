//! Kernel-owned signed Agent image trust records.
//!
//! This core-layer module defines fixed-width signer identity, scope, status,
//! rotation receipts, and replay evidence. It performs only signer-ID hashing;
//! Ed25519 parsing and signature arithmetic remain in the machine loader.

use sha2::{Digest, Sha256};

use crate::{AgentImageKind, ResourceId};

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
    bits: u16,
}

impl AgentImageKindScope {
    const WORKER: u16 = 1 << 0;
    const VERIFIER: u16 = 1 << 1;
    const FAULT_HANDLER: u16 = 1 << 2;
    const SUPERVISOR: u16 = 1 << 3;
    const KNOWN_BITS: u16 = Self::WORKER | Self::VERIFIER | Self::FAULT_HANDLER | Self::SUPERVISOR;

    pub const fn only(kind: AgentImageKind) -> Self {
        let bits = match kind {
            AgentImageKind::Worker => Self::WORKER,
            AgentImageKind::Verifier => Self::VERIFIER,
            AgentImageKind::FaultHandler => Self::FAULT_HANDLER,
            AgentImageKind::Supervisor => Self::SUPERVISOR,
            AgentImageKind::Bootstrap | AgentImageKind::Driver => 0,
        };
        Self { bits }
    }

    pub const fn all() -> Self {
        Self {
            bits: Self::KNOWN_BITS,
        }
    }

    pub const fn from_bits(bits: u16) -> Option<Self> {
        if bits != 0 && bits & !Self::KNOWN_BITS == 0 {
            Some(Self { bits })
        } else {
            None
        }
    }

    pub const fn bits(self) -> u16 {
        self.bits
    }

    pub const fn allows(self, kind: AgentImageKind) -> bool {
        self.bits & Self::only(kind).bits != 0
    }

    pub(crate) const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.bits == 0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageSignerStatus {
    Active,
    Revoked,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageSignerRecord {
    pub signer_id: AgentImageSignerId,
    pub resource: ResourceId,
    pub public_key: [u8; 32],
    pub image_kinds: AgentImageKindScope,
    pub minimum_abi: u16,
    pub maximum_abi: u16,
    pub status: AgentImageSignerStatus,
    pub generation: u64,
}

impl AgentImageSignerRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            signer_id: AgentImageSignerId::new([0; 32]),
            resource: ResourceId::new(0),
            public_key: [0; 32],
            image_kinds: AgentImageKindScope::empty(),
            minimum_abi: 0,
            maximum_abi: 0,
            status: AgentImageSignerStatus::Revoked,
            generation: 0,
        }
    }

    pub fn allows(self, kind: AgentImageKind, abi_version: u16) -> bool {
        self.status == AgentImageSignerStatus::Active
            && self.image_kinds.allows(kind)
            && abi_version >= self.minimum_abi
            && abi_version <= self.maximum_abi
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageSignerEvent {
    pub signer_id: AgentImageSignerId,
    pub peer_signer_id: Option<AgentImageSignerId>,
    pub public_key: [u8; 32],
    pub image_kinds: AgentImageKindScope,
    pub minimum_abi: u16,
    pub maximum_abi: u16,
    pub status: AgentImageSignerStatus,
    pub policy_generation: u64,
}

impl AgentImageSignerEvent {
    pub(crate) const fn from_record(
        record: AgentImageSignerRecord,
        peer_signer_id: Option<AgentImageSignerId>,
    ) -> Self {
        Self {
            signer_id: record.signer_id,
            peer_signer_id,
            public_key: record.public_key,
            image_kinds: record.image_kinds,
            minimum_abi: record.minimum_abi,
            maximum_abi: record.maximum_abi,
            status: record.status,
            policy_generation: record.generation,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentImageSignerRotation {
    previous: AgentImageSignerRecord,
    replacement: AgentImageSignerRecord,
    generation: u64,
}

impl AgentImageSignerRotation {
    pub(crate) const fn new(
        previous: AgentImageSignerRecord,
        replacement: AgentImageSignerRecord,
        generation: u64,
    ) -> Self {
        Self {
            previous,
            replacement,
            generation,
        }
    }

    pub const fn previous(self) -> AgentImageSignerRecord {
        self.previous
    }

    pub const fn replacement(self) -> AgentImageSignerRecord {
        self.replacement
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}
