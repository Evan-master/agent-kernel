//! Structurally validated durable Event Archive manifest values.
//!
//! This Core child binds one archive proposal to authority, storage, payload,
//! State Signer policy, and optional trusted-anchor evidence. It owns bounds
//! and identity checks while leaving canonical encoding and cryptography out.

use crate::{
    AgentId, CapabilityId, EventArchiveDigest, EventArchiveProposal, ResourceId,
    MAX_DURABLE_ARCHIVE_BYTES, MAX_DURABLE_ARCHIVE_EVENTS,
};

use super::{DurableArchiveAnchor, DurableStateDigest, DurableStateSignerId};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveManifestError {
    EventCountOutOfRange { count: usize, limit: usize },
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
        let payload_limit = MAX_DURABLE_ARCHIVE_BYTES as u32;
        if payload_length == 0 || payload_length > payload_limit {
            return Err(DurableArchiveManifestError::PayloadLengthOutOfRange {
                length: payload_length,
                limit: payload_limit,
            });
        }
        if payload_digest.bytes() != proposal.digest().bytes {
            return Err(DurableArchiveManifestError::PayloadDigestMismatch);
        }
        if actor.raw() == 0 {
            return Err(DurableArchiveManifestError::ZeroActor);
        }
        if archive_authority.raw() == 0 {
            return Err(DurableArchiveManifestError::ZeroArchiveAuthority);
        }
        if root.raw() == 0 {
            return Err(DurableArchiveManifestError::ZeroRootResource);
        }
        if storage.raw() == 0 {
            return Err(DurableArchiveManifestError::ZeroStorageResource);
        }
        if signer_id.is_zero() {
            return Err(DurableArchiveManifestError::ZeroSignerId);
        }
        if signer_policy_generation == 0 {
            return Err(DurableArchiveManifestError::ZeroSignerPolicyGeneration);
        }
        if !anchor.matches_previous(proposal.generation(), proposal.previous_digest()) {
            return Err(DurableArchiveManifestError::AnchorMismatch);
        }

        Ok(Self {
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
