//! Structurally validated durable Event Archive manifest values.
//!
//! This Core child binds one archive proposal to authority, storage, payload,
//! State Signer policy, and optional trusted-anchor evidence. It owns bounds
//! and identity checks while leaving canonical encoding and cryptography out.

mod fields;

pub use fields::DurableArchiveManifestFields;

use crate::{
    AgentId, CapabilityId, EventArchiveDigest, EventArchiveProposal, ResourceId,
    MAX_DURABLE_ARCHIVE_EVENTS,
};

use super::{DurableArchiveAnchor, DurableStateDigest, DurableStateSignerId};

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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proposal: EventArchiveProposal,
        actor: AgentId,
        archive_authority: CapabilityId,
        root: ResourceId,
        storage: ResourceId,
        payload_length: u32,
        payload_digest: DurableStateDigest,
        signer_id: DurableStateSignerId,
        signer_policy_generation: u64,
        anchor: DurableArchiveAnchor,
    ) -> Result<Self, DurableArchiveManifestError> {
        let count = proposal.count();
        if count == 0 || count > MAX_DURABLE_ARCHIVE_EVENTS {
            return Err(DurableArchiveManifestError::EventCountOutOfRange {
                count,
                limit: MAX_DURABLE_ARCHIVE_EVENTS,
            });
        }
        Self::from_fields(DurableArchiveManifestFields {
            generation: proposal.generation(),
            first_sequence: proposal.first_sequence(),
            through_sequence: proposal.through_sequence(),
            event_count: count as u16,
            previous_digest: proposal.previous_digest(),
            archive_digest: proposal.digest(),
            actor,
            archive_authority,
            root,
            storage,
            payload_length,
            payload_digest,
            signer_id,
            signer_policy_generation,
            anchor,
        })
    }

    pub fn from_fields(
        fields: DurableArchiveManifestFields,
    ) -> Result<Self, DurableArchiveManifestError> {
        fields::validate(fields)?;
        Ok(Self {
            generation: fields.generation,
            first_sequence: fields.first_sequence,
            through_sequence: fields.through_sequence,
            event_count: fields.event_count,
            previous_digest: fields.previous_digest,
            archive_digest: fields.archive_digest,
            actor: fields.actor,
            archive_authority: fields.archive_authority,
            root: fields.root,
            storage: fields.storage,
            payload_length: fields.payload_length,
            payload_digest: fields.payload_digest,
            signer_id: fields.signer_id,
            signer_policy_generation: fields.signer_policy_generation,
            anchor: fields.anchor,
        })
    }

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
