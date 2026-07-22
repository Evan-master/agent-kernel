//! Binding between parsed capsule bytes and kernel-verified image metadata.

use agent_kernel_core::{AgentImageKind, AgentImageRecord, AgentImageStatus};

use super::{
    sha256_digest, AgentImageCapsule, AgentImageFormat, AgentImageLoadError, AgentImageRelocation,
    AgentImageSignerId, AgentImageTrustPolicy, AGENT_IMAGE_KIND_FAULT_HANDLER,
    AGENT_IMAGE_KIND_SUPERVISOR, AGENT_IMAGE_KIND_VERIFIER, AGENT_IMAGE_KIND_WORKER,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageTrust {
    DigestPinned,
    Signed(AgentImageSignerId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VerifiedAgentImage<'a> {
    record: AgentImageRecord,
    capsule: AgentImageCapsule<'a>,
    trust: AgentImageTrust,
}

impl<'a> VerifiedAgentImage<'a> {
    pub fn verify(record: AgentImageRecord, bytes: &'a [u8]) -> Result<Self, AgentImageLoadError> {
        let capsule = verify_record(record, bytes)?;
        if capsule.format() == AgentImageFormat::SignedPackageV3 {
            return Err(AgentImageLoadError::SignatureVerificationRequired);
        }
        Ok(Self {
            record,
            capsule,
            trust: AgentImageTrust::DigestPinned,
        })
    }

    pub fn verify_signed<const N: usize>(
        record: AgentImageRecord,
        bytes: &'a [u8],
        policy: &AgentImageTrustPolicy<N>,
    ) -> Result<Self, AgentImageLoadError> {
        let capsule = verify_record(record, bytes)?;
        if capsule.format() != AgentImageFormat::SignedPackageV3 {
            return Err(AgentImageLoadError::SignatureRequired);
        }
        let signer_id = policy.verify(&capsule)?;
        Ok(Self {
            record,
            capsule,
            trust: AgentImageTrust::Signed(signer_id),
        })
    }

    pub const fn record(&self) -> AgentImageRecord {
        self.record
    }

    pub const fn code(&self) -> &'a [u8] {
        self.capsule.code()
    }

    pub const fn format(&self) -> AgentImageFormat {
        self.capsule.format()
    }

    pub const fn rodata(&self) -> &'a [u8] {
        self.capsule.rodata()
    }

    pub const fn signer_id(&self) -> Option<AgentImageSignerId> {
        self.capsule.signer_id()
    }

    pub const fn trust(&self) -> AgentImageTrust {
        self.trust
    }

    pub const fn entry_offset(&self) -> u32 {
        self.capsule.entry_offset()
    }

    pub const fn code_page_count(&self) -> usize {
        self.capsule.code_page_count()
    }

    pub const fn rodata_page_count(&self) -> usize {
        self.capsule.rodata_page_count()
    }

    pub const fn relocation_count(&self) -> usize {
        self.capsule.relocation_count()
    }

    pub fn relocation(&self, index: usize) -> Option<AgentImageRelocation> {
        self.capsule.relocation(index)
    }
}

fn verify_record<'a>(
    record: AgentImageRecord,
    bytes: &'a [u8],
) -> Result<AgentImageCapsule<'a>, AgentImageLoadError> {
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
    Ok(capsule)
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
