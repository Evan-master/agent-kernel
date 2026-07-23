//! Canonical fixed-width signing message for one durable archive manifest.
//!
//! The machine layer writes every logical field explicitly in little-endian
//! order. Rust layout and pointer width never enter the 285-byte message; flags
//! and reserved bytes stay frozen for later capsule parsing.

use agent_kernel_core::{DurableAnchorMode, DurableArchiveManifest, DurableStateDigest};
use sha2::{Digest, Sha256};

const DOMAIN: &[u8; 29] = b"AGENT-KERNEL-DURABLE-ARCHIVE\0";
const MANIFEST_BODY_BYTES: usize = 256;
const TRUSTED_ANCHOR_FLAG: u16 = 1;

pub const DURABLE_ARCHIVE_MANIFEST_FORMAT_VERSION: u16 = 1;
pub const DURABLE_ARCHIVE_MANIFEST_BYTES: usize = DOMAIN.len() + MANIFEST_BODY_BYTES;

pub fn encode_durable_archive_manifest(
    manifest: DurableArchiveManifest,
) -> [u8; DURABLE_ARCHIVE_MANIFEST_BYTES] {
    let mut encoder = ManifestEncoder::new();
    encoder.put(DOMAIN);
    encoder.put(&DURABLE_ARCHIVE_MANIFEST_FORMAT_VERSION.to_le_bytes());
    let flags = match manifest.anchor().mode() {
        DurableAnchorMode::Unanchored => 0,
        DurableAnchorMode::Trusted => TRUSTED_ANCHOR_FLAG,
    };
    encoder.put(&flags.to_le_bytes());
    encoder.put(&[0; 4]);
    encoder.put(&manifest.generation().to_le_bytes());
    encoder.put(&manifest.first_sequence().to_le_bytes());
    encoder.put(&manifest.through_sequence().to_le_bytes());
    encoder.put(&manifest.event_count().to_le_bytes());
    encoder.put(&[0; 6]);
    encoder.put(&manifest.previous_digest().bytes);
    encoder.put(&manifest.archive_digest().bytes);
    encoder.put(&manifest.actor().raw().to_le_bytes());
    encoder.put(&manifest.archive_authority().raw().to_le_bytes());
    encoder.put(&manifest.root().raw().to_le_bytes());
    encoder.put(&manifest.storage().raw().to_le_bytes());
    encoder.put(&manifest.payload_length().to_le_bytes());
    encoder.put(&[0; 4]);
    encoder.put(&manifest.payload_digest().bytes());
    encoder.put(&manifest.signer_id().bytes());
    encoder.put(&manifest.signer_policy_generation().to_le_bytes());
    encoder.put(&manifest.anchor().generation().to_le_bytes());
    encoder.put(&manifest.anchor().digest().bytes);
    debug_assert_eq!(encoder.offset, DURABLE_ARCHIVE_MANIFEST_BYTES);
    encoder.bytes
}

pub fn durable_archive_manifest_digest(manifest: DurableArchiveManifest) -> DurableStateDigest {
    let digest = Sha256::digest(encode_durable_archive_manifest(manifest));
    DurableStateDigest::new(digest.into())
}

struct ManifestEncoder {
    bytes: [u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    offset: usize,
}

impl ManifestEncoder {
    const fn new() -> Self {
        Self {
            bytes: [0; DURABLE_ARCHIVE_MANIFEST_BYTES],
            offset: 0,
        }
    }

    fn put(&mut self, bytes: &[u8]) {
        let end = self.offset + bytes.len();
        self.bytes[self.offset..end].copy_from_slice(bytes);
        self.offset = end;
    }
}
