//! Validation boundary for manifest fields reconstructed from durable bytes.

use crate::{
    AgentId, CapabilityId, EventArchiveDigest, ResourceId, MAX_DURABLE_ARCHIVE_EVENTS,
    MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES,
};

use super::{
    DurableArchiveAnchor, DurableArchiveManifestError, DurableStateDigest, DurableStateSignerId,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveManifestFields {
    pub generation: u64,
    pub first_sequence: u64,
    pub through_sequence: u64,
    pub event_count: u16,
    pub previous_digest: EventArchiveDigest,
    pub archive_digest: EventArchiveDigest,
    pub actor: AgentId,
    pub archive_authority: CapabilityId,
    pub root: ResourceId,
    pub storage: ResourceId,
    pub payload_length: u32,
    pub payload_digest: DurableStateDigest,
    pub signer_id: DurableStateSignerId,
    pub signer_policy_generation: u64,
    pub anchor: DurableArchiveAnchor,
}

pub(super) fn validate(
    fields: DurableArchiveManifestFields,
) -> Result<(), DurableArchiveManifestError> {
    if fields.generation == 0 {
        return Err(DurableArchiveManifestError::ZeroGeneration);
    }

    let count = usize::from(fields.event_count);
    if count == 0 || count > MAX_DURABLE_ARCHIVE_EVENTS {
        return Err(DurableArchiveManifestError::EventCountOutOfRange {
            count,
            limit: MAX_DURABLE_ARCHIVE_EVENTS,
        });
    }
    let expected_through = fields
        .first_sequence
        .checked_add(u64::from(fields.event_count) - 1);
    if fields.first_sequence == 0 || expected_through != Some(fields.through_sequence) {
        return Err(DurableArchiveManifestError::SequenceRangeMismatch);
    }
    if fields.generation == 1
        && (fields.first_sequence != 1
            || fields.previous_digest.bytes != EventArchiveDigest::ZERO.bytes)
    {
        return Err(DurableArchiveManifestError::GenesisMismatch);
    }

    let payload_limit = MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES as u32;
    if fields.payload_length == 0 || fields.payload_length > payload_limit {
        return Err(DurableArchiveManifestError::PayloadLengthOutOfRange {
            length: fields.payload_length,
            limit: payload_limit,
        });
    }
    if fields.payload_digest.bytes() != fields.archive_digest.bytes {
        return Err(DurableArchiveManifestError::PayloadDigestMismatch);
    }
    if fields.actor.raw() == 0 {
        return Err(DurableArchiveManifestError::ZeroActor);
    }
    if fields.archive_authority.raw() == 0 {
        return Err(DurableArchiveManifestError::ZeroArchiveAuthority);
    }
    if fields.root.raw() == 0 {
        return Err(DurableArchiveManifestError::ZeroRootResource);
    }
    if fields.storage.raw() == 0 {
        return Err(DurableArchiveManifestError::ZeroStorageResource);
    }
    if fields.signer_id.is_zero() {
        return Err(DurableArchiveManifestError::ZeroSignerId);
    }
    if fields.signer_policy_generation == 0 {
        return Err(DurableArchiveManifestError::ZeroSignerPolicyGeneration);
    }
    if !fields
        .anchor
        .matches_previous(fields.generation, fields.previous_digest)
    {
        return Err(DurableArchiveManifestError::AnchorMismatch);
    }
    Ok(())
}
