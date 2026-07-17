//! Binding between parsed capsule bytes and kernel-verified image metadata.

use agent_kernel_core::{AgentImageKind, AgentImageRecord, AgentImageStatus};

use super::{
    sha256_digest, AgentImageCapsule, AgentImageLoadError, AGENT_IMAGE_KIND_FAULT_HANDLER,
    AGENT_IMAGE_KIND_SUPERVISOR, AGENT_IMAGE_KIND_VERIFIER, AGENT_IMAGE_KIND_WORKER,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VerifiedAgentImage<'a> {
    record: AgentImageRecord,
    capsule: AgentImageCapsule<'a>,
}

impl<'a> VerifiedAgentImage<'a> {
    pub fn verify(record: AgentImageRecord, bytes: &'a [u8]) -> Result<Self, AgentImageLoadError> {
        let capsule = AgentImageCapsule::parse(bytes)?;
        if record.status != AgentImageStatus::Verified {
            return Err(AgentImageLoadError::ImageNotVerified);
        }
        let header = capsule.header();
        if !image_kind_matches(record.kind, header.image_kind())
            || record.abi_version != header.abi_version()
            || record.entry_version != header.entry_version()
        {
            return Err(AgentImageLoadError::MetadataMismatch);
        }
        if record.digest != sha256_digest(capsule.raw()) {
            return Err(AgentImageLoadError::DigestMismatch);
        }
        Ok(Self { record, capsule })
    }

    pub const fn record(&self) -> AgentImageRecord {
        self.record
    }

    pub const fn code(&self) -> &'a [u8] {
        self.capsule.code()
    }

    pub const fn entry_offset(&self) -> u32 {
        self.capsule.entry_offset()
    }
}

fn image_kind_matches(record: AgentImageKind, header: u16) -> bool {
    matches!(
        (record, header),
        (AgentImageKind::Worker, AGENT_IMAGE_KIND_WORKER)
            | (AgentImageKind::Verifier, AGENT_IMAGE_KIND_VERIFIER)
            | (AgentImageKind::FaultHandler, AGENT_IMAGE_KIND_FAULT_HANDLER)
            | (AgentImageKind::Supervisor, AGENT_IMAGE_KIND_SUPERVISOR)
    )
}
