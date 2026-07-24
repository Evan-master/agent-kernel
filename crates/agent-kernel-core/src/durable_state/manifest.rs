//! Structurally validated durable Event Archive manifest values.
//!
//! This Core child binds one archive proposal to authority, storage, payload,
//! State Signer policy, and optional trusted-anchor evidence. It owns bounds
//! and identity checks while leaving canonical encoding and cryptography out.

mod construction;
mod fields;

pub use fields::DurableArchiveManifestFields;

use crate::{AgentId, CapabilityId, EventArchiveDigest, ResourceId};

use super::{
    DurableArchiveAnchor, DurableSignatureAlgorithm, DurableStateDigest, DurableStateSignerId,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum DurableArchiveManifestVersion {
    LegacyEd25519 = 1,
    AlgorithmBound = 2,
}

impl DurableArchiveManifestVersion {
    pub const fn wire_value(self) -> u16 {
        self as u16
    }

    pub const fn from_wire_value(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::LegacyEd25519),
            2 => Some(Self::AlgorithmBound),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveManifestError {
    ZeroGeneration,
    EventCountOutOfRange { count: usize, limit: usize },
    SequenceRangeMismatch,
    GenesisMismatch,
    PayloadLengthOutOfRange { length: u32, limit: u32 },
    PayloadDigestMismatch,
    ZeroActor,
    ZeroArchiveAuthority,
    ZeroRootResource,
    ZeroStorageResource,
    ZeroSignerId,
    ZeroSignerPolicyGeneration,
    AnchorMismatch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveManifest {
    version: DurableArchiveManifestVersion,
    signature_algorithm: DurableSignatureAlgorithm,
    generation: u64,
    first_sequence: u64,
    through_sequence: u64,
    event_count: u16,
    previous_digest: EventArchiveDigest,
    archive_digest: EventArchiveDigest,
    actor: AgentId,
    archive_authority: CapabilityId,
    root: ResourceId,
    storage: ResourceId,
    payload_length: u32,
    payload_digest: DurableStateDigest,
    signer_id: DurableStateSignerId,
    signer_policy_generation: u64,
    anchor: DurableArchiveAnchor,
}

impl DurableArchiveManifest {
    pub const fn fields(self) -> DurableArchiveManifestFields {
        DurableArchiveManifestFields {
            generation: self.generation,
            first_sequence: self.first_sequence,
            through_sequence: self.through_sequence,
            event_count: self.event_count,
            previous_digest: self.previous_digest,
            archive_digest: self.archive_digest,
            actor: self.actor,
            archive_authority: self.archive_authority,
            root: self.root,
            storage: self.storage,
            payload_length: self.payload_length,
            payload_digest: self.payload_digest,
            signer_id: self.signer_id,
            signer_policy_generation: self.signer_policy_generation,
            anchor: self.anchor,
        }
    }

    pub const fn version(self) -> DurableArchiveManifestVersion {
        self.version
    }

    pub const fn signature_algorithm(self) -> DurableSignatureAlgorithm {
        self.signature_algorithm
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn first_sequence(self) -> u64 {
        self.first_sequence
    }

    pub const fn through_sequence(self) -> u64 {
        self.through_sequence
    }

    pub const fn event_count(self) -> u16 {
        self.event_count
    }

    pub const fn previous_digest(self) -> EventArchiveDigest {
        self.previous_digest
    }

    pub const fn archive_digest(self) -> EventArchiveDigest {
        self.archive_digest
    }

    pub const fn actor(self) -> AgentId {
        self.actor
    }

    pub const fn archive_authority(self) -> CapabilityId {
        self.archive_authority
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }

    pub const fn storage(self) -> ResourceId {
        self.storage
    }

    pub const fn payload_length(self) -> u32 {
        self.payload_length
    }

    pub const fn payload_digest(self) -> DurableStateDigest {
        self.payload_digest
    }

    pub const fn signer_id(self) -> DurableStateSignerId {
        self.signer_id
    }

    pub const fn signer_policy_generation(self) -> u64 {
        self.signer_policy_generation
    }

    pub const fn anchor(self) -> DurableArchiveAnchor {
        self.anchor
    }
}
