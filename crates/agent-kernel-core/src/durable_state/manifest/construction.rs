//! Durable archive manifest construction and structural validation.
//!
//! This Core-layer module converts proposals or decoded fields into immutable
//! manifests. It preserves version/algorithm mappings, enforces fixed bounds,
//! and performs no encoding, allocation, storage I/O, or cryptography.

use crate::{AgentId, CapabilityId, EventArchiveProposal, ResourceId, MAX_DURABLE_ARCHIVE_EVENTS};

use super::super::{
    DurableArchiveAnchor, DurableSignatureAlgorithm, DurableStateDigest, DurableStateSignerId,
};
use super::{
    fields, DurableArchiveManifest, DurableArchiveManifestError, DurableArchiveManifestFields,
    DurableArchiveManifestVersion,
};

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
        Self::from_proposal(
            proposal,
            actor,
            archive_authority,
            root,
            storage,
            payload_length,
            payload_digest,
            signer_id,
            signer_policy_generation,
            anchor,
            DurableArchiveManifestVersion::LegacyEd25519,
            DurableSignatureAlgorithm::Ed25519,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_algorithm_bound(
        proposal: EventArchiveProposal,
        actor: AgentId,
        archive_authority: CapabilityId,
        root: ResourceId,
        storage: ResourceId,
        payload_length: u32,
        payload_digest: DurableStateDigest,
        signer_id: DurableStateSignerId,
        signature_algorithm: DurableSignatureAlgorithm,
        signer_policy_generation: u64,
        anchor: DurableArchiveAnchor,
    ) -> Result<Self, DurableArchiveManifestError> {
        Self::from_proposal(
            proposal,
            actor,
            archive_authority,
            root,
            storage,
            payload_length,
            payload_digest,
            signer_id,
            signer_policy_generation,
            anchor,
            DurableArchiveManifestVersion::AlgorithmBound,
            signature_algorithm,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_for_signature_algorithm(
        proposal: EventArchiveProposal,
        actor: AgentId,
        archive_authority: CapabilityId,
        root: ResourceId,
        storage: ResourceId,
        payload_length: u32,
        payload_digest: DurableStateDigest,
        signer_id: DurableStateSignerId,
        signature_algorithm: DurableSignatureAlgorithm,
        signer_policy_generation: u64,
        anchor: DurableArchiveAnchor,
    ) -> Result<Self, DurableArchiveManifestError> {
        match signature_algorithm {
            DurableSignatureAlgorithm::Ed25519 => Self::new(
                proposal,
                actor,
                archive_authority,
                root,
                storage,
                payload_length,
                payload_digest,
                signer_id,
                signer_policy_generation,
                anchor,
            ),
            DurableSignatureAlgorithm::EcdsaP256Sha256 => Self::new_algorithm_bound(
                proposal,
                actor,
                archive_authority,
                root,
                storage,
                payload_length,
                payload_digest,
                signer_id,
                signature_algorithm,
                signer_policy_generation,
                anchor,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn from_proposal(
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
        version: DurableArchiveManifestVersion,
        signature_algorithm: DurableSignatureAlgorithm,
    ) -> Result<Self, DurableArchiveManifestError> {
        let count = proposal.count();
        if count == 0 || count > MAX_DURABLE_ARCHIVE_EVENTS {
            return Err(DurableArchiveManifestError::EventCountOutOfRange {
                count,
                limit: MAX_DURABLE_ARCHIVE_EVENTS,
            });
        }
        Self::from_parts(
            DurableArchiveManifestFields {
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
            },
            version,
            signature_algorithm,
        )
    }

    pub fn from_fields(
        fields: DurableArchiveManifestFields,
    ) -> Result<Self, DurableArchiveManifestError> {
        Self::from_parts(
            fields,
            DurableArchiveManifestVersion::LegacyEd25519,
            DurableSignatureAlgorithm::Ed25519,
        )
    }

    pub fn from_algorithm_bound_fields(
        fields: DurableArchiveManifestFields,
        signature_algorithm: DurableSignatureAlgorithm,
    ) -> Result<Self, DurableArchiveManifestError> {
        Self::from_parts(
            fields,
            DurableArchiveManifestVersion::AlgorithmBound,
            signature_algorithm,
        )
    }

    fn from_parts(
        fields: DurableArchiveManifestFields,
        version: DurableArchiveManifestVersion,
        signature_algorithm: DurableSignatureAlgorithm,
    ) -> Result<Self, DurableArchiveManifestError> {
        fields::validate(fields)?;
        Ok(Self {
            version,
            signature_algorithm,
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
}
