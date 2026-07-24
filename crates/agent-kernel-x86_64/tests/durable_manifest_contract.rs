#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{
    DurableAnchorMode, DurableArchiveAnchor, DurableArchiveManifestVersion,
    DurableSignatureAlgorithm, DurableStateDigest,
};
use agent_kernel_x86_64::durable_state::{
    decode_durable_archive_manifest, durable_archive_manifest_digest,
    encode_durable_archive_manifest, DurableArchiveManifestDecodeError,
    DURABLE_ARCHIVE_MANIFEST_BYTES,
};
use sha2::{Digest, Sha256};

use durable_state_support::{
    manifest, p256_manifest, p256_signing_key, signing_key, POLICY_GENERATION, ROOT, STORAGE,
};

#[test]
fn canonical_manifest_has_one_frozen_285_byte_encoding() {
    let signing_key = signing_key(0x11);
    let manifest = manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let encoded = encode_durable_archive_manifest(manifest);
    let expected = expected_bytes(manifest);

    assert_eq!(DURABLE_ARCHIVE_MANIFEST_BYTES, 285);
    assert_eq!(encoded.as_slice(), expected);
    assert_eq!(
        durable_archive_manifest_digest(manifest),
        DurableStateDigest::new(Sha256::digest(encoded).into())
    );
}

#[test]
fn algorithm_bound_manifest_uses_v2_and_the_reserved_algorithm_slot() {
    let signing_key = p256_signing_key(0x12);
    let manifest = p256_manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );

    let encoded = encode_durable_archive_manifest(manifest);

    assert_eq!(
        manifest.version(),
        DurableArchiveManifestVersion::AlgorithmBound
    );
    assert_eq!(
        manifest.signature_algorithm(),
        DurableSignatureAlgorithm::EcdsaP256Sha256
    );
    assert_eq!(&encoded[29..31], &2u16.to_le_bytes());
    assert_eq!(&encoded[33..35], &2u16.to_le_bytes());
    assert_eq!(&encoded[35..37], &[0, 0]);
    assert_eq!(
        agent_kernel_x86_64::durable_state::decode_durable_archive_manifest(&encoded),
        Ok(manifest)
    );
}

#[test]
fn versions_reject_noncanonical_or_unknown_algorithm_encodings() {
    let legacy = manifest(
        &signing_key(0x13),
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let mut legacy_bytes = encode_durable_archive_manifest(legacy);
    legacy_bytes[33..35].copy_from_slice(&1u16.to_le_bytes());
    assert_eq!(
        decode_durable_archive_manifest(&legacy_bytes),
        Err(DurableArchiveManifestDecodeError::ReservedNotZero)
    );

    let algorithm_bound = p256_manifest(
        &p256_signing_key(0x14),
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let mut algorithm_bytes = encode_durable_archive_manifest(algorithm_bound);
    algorithm_bytes[33..35].copy_from_slice(&3u16.to_le_bytes());
    assert_eq!(
        decode_durable_archive_manifest(&algorithm_bytes),
        Err(DurableArchiveManifestDecodeError::UnsupportedSignatureAlgorithm { algorithm: 3 })
    );

    algorithm_bytes[29..31].copy_from_slice(&3u16.to_le_bytes());
    assert_eq!(
        decode_durable_archive_manifest(&algorithm_bytes),
        Err(DurableArchiveManifestDecodeError::UnsupportedVersion { version: 3 })
    );
}

fn expected_bytes(manifest: agent_kernel_core::DurableArchiveManifest) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"AGENT-KERNEL-DURABLE-ARCHIVE\0");
    bytes.extend_from_slice(&1u16.to_le_bytes());
    let flags = match manifest.anchor().mode() {
        DurableAnchorMode::Unanchored => 0u16,
        DurableAnchorMode::Trusted => 1u16,
    };
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&manifest.generation().to_le_bytes());
    bytes.extend_from_slice(&manifest.first_sequence().to_le_bytes());
    bytes.extend_from_slice(&manifest.through_sequence().to_le_bytes());
    bytes.extend_from_slice(&manifest.event_count().to_le_bytes());
    bytes.extend_from_slice(&[0; 6]);
    bytes.extend_from_slice(&manifest.previous_digest().bytes);
    bytes.extend_from_slice(&manifest.archive_digest().bytes);
    bytes.extend_from_slice(&manifest.actor().raw().to_le_bytes());
    bytes.extend_from_slice(&manifest.archive_authority().raw().to_le_bytes());
    bytes.extend_from_slice(&manifest.root().raw().to_le_bytes());
    bytes.extend_from_slice(&manifest.storage().raw().to_le_bytes());
    bytes.extend_from_slice(&manifest.payload_length().to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&manifest.payload_digest().bytes());
    bytes.extend_from_slice(&manifest.signer_id().bytes());
    bytes.extend_from_slice(&manifest.signer_policy_generation().to_le_bytes());
    bytes.extend_from_slice(&manifest.anchor().generation().to_le_bytes());
    bytes.extend_from_slice(&manifest.anchor().digest().bytes);
    bytes
}
