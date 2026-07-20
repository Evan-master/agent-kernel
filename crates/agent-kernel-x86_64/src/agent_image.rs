//! Native x86_64 Agent Image Capsule validation.
//!
//! This architecture-library module parses the bounded AgentOS image format,
//! computes its no_std SHA-256 identity, and binds immutable bytes to a verified
//! kernel image record. Mapping and execution remain binary-layer concerns.

mod format;
mod verification;

use agent_kernel_core::AgentImageDigest;
use sha2::{Digest, Sha256};

pub use format::{AgentImageCapsule, AgentImageHeader};
pub use verification::VerifiedAgentImage;

pub const AGENT_IMAGE_HEADER_BYTES: usize = 32;
pub const MAX_AGENT_CODE_PAGES: usize = crate::address_space::AGENT_CODE_PAGE_COUNT;
pub const MAX_AGENT_CODE_BYTES: usize =
    crate::user_memory::PAGE_BYTES as usize * MAX_AGENT_CODE_PAGES;

pub(crate) const AGENT_IMAGE_MAGIC: &[u8; 8] = b"AGNTIMG\0";
pub(crate) const AGENT_IMAGE_FORMAT_VERSION: u16 = 1;
pub(crate) const AGENT_IMAGE_ARCH_X86_64: u16 = 1;
pub(crate) const AGENT_IMAGE_KIND_WORKER: u16 = 1;
pub(crate) const AGENT_IMAGE_KIND_VERIFIER: u16 = 2;
pub(crate) const AGENT_IMAGE_KIND_FAULT_HANDLER: u16 = 3;
pub(crate) const AGENT_IMAGE_KIND_SUPERVISOR: u16 = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentImageLoadError {
    HeaderTruncated,
    InvalidMagic,
    UnsupportedFormatVersion,
    UnsupportedArchitecture,
    UnsupportedImageKind,
    UnsupportedFlags,
    InvalidVersion,
    ReservedNotZero,
    InvalidCodeLength,
    LengthMismatch,
    EntryOutOfRange,
    ImageNotVerified,
    MetadataMismatch,
    DigestMismatch,
}

pub fn sha256_digest(bytes: &[u8]) -> AgentImageDigest {
    let output = Sha256::digest(bytes);
    let mut digest = [0; 32];
    digest.copy_from_slice(&output);
    AgentImageDigest::new(digest)
}
