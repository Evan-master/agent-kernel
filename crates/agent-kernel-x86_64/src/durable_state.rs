//! Native signed durable-state manifest verification.
//!
//! This x86_64 machine layer owns the canonical signing message and strict
//! Ed25519 verification against a read-only State Signer policy. Storage I/O,
//! private keys, and Core Event release remain outside this module.

mod manifest;
mod trust;

pub use agent_kernel_core::DURABLE_ARCHIVE_MANIFEST_BYTES;
pub use manifest::{
    durable_archive_manifest_digest, encode_durable_archive_manifest,
    DURABLE_ARCHIVE_MANIFEST_FORMAT_VERSION,
};
pub use trust::{
    DurableStateTrustPolicy, DurableStateVerificationError, VerifiedDurableArchiveManifest,
};
