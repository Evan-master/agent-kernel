use agent_kernel_core::{
    durable_state_signer_id, durable_state_signer_id_for_key, AgentId, CapabilityId,
    DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveManifestVersion,
    DurableSignatureAlgorithm, DurableStateDigest, DurableStatePublicKey, DurableStateSignerRecord,
    DurableStateSignerStatus, EventArchiveProposal, KernelCore, ResourceId,
};
use sha2::{Digest, Sha256};

type TestCore = KernelCore<1, 0, 0, 2, 0, 0, 0, 0, 0, 0>;

#[test]
fn signature_algorithm_and_manifest_versions_have_stable_values() {
    assert_eq!(DurableSignatureAlgorithm::Ed25519.wire_value(), 1);
    assert_eq!(DurableSignatureAlgorithm::EcdsaP256Sha256.wire_value(), 2);
    assert_eq!(
        DurableSignatureAlgorithm::from_wire_value(1),
        Some(DurableSignatureAlgorithm::Ed25519)
    );
    assert_eq!(
        DurableSignatureAlgorithm::from_wire_value(2),
        Some(DurableSignatureAlgorithm::EcdsaP256Sha256)
    );
    assert_eq!(DurableSignatureAlgorithm::from_wire_value(0), None);
    assert_eq!(DurableSignatureAlgorithm::from_wire_value(3), None);

    assert_eq!(DurableArchiveManifestVersion::LegacyEd25519.wire_value(), 1);
    assert_eq!(
        DurableArchiveManifestVersion::AlgorithmBound.wire_value(),
        2
    );
}

#[test]
fn public_keys_validate_exact_algorithm_specific_encodings() {
    let ed25519 = DurableStatePublicKey::ed25519([0x31; 32]);
    assert_eq!(ed25519.algorithm(), DurableSignatureAlgorithm::Ed25519);
    assert_eq!(ed25519.ed25519_bytes(), Some([0x31; 32]));
    assert_eq!(ed25519.ecdsa_p256_bytes(), None);

    let mut compressed = [0x42; 33];
    compressed[0] = 0x02;
    let p256 = DurableStatePublicKey::ecdsa_p256(compressed).expect("canonical compressed key");
    assert_eq!(p256.algorithm(), DurableSignatureAlgorithm::EcdsaP256Sha256);
    assert_eq!(p256.ecdsa_p256_bytes(), Some(compressed));
    assert_eq!(p256.ed25519_bytes(), None);

    compressed[0] = 0x04;
    assert_eq!(DurableStatePublicKey::ecdsa_p256(compressed), None);
    compressed[0] = 0;
    assert_eq!(DurableStatePublicKey::ecdsa_p256(compressed), None);
    assert!(DurableStateSignerRecord::new_with_key(
        ResourceId::new(3),
        DurableStatePublicKey::EcdsaP256(compressed),
        DurableStateSignerStatus::Active,
        7,
    )
    .is_none());
}

#[test]
fn legacy_ed25519_id_is_unchanged_and_p256_id_is_algorithm_bound() {
    let ed25519 = [0x53; 32];
    let expected_legacy = Sha256::digest(
        [
            b"AGENT-KERNEL-DURABLE-STATE-SIGNER-V1\0".as_slice(),
            ed25519.as_slice(),
        ]
        .concat(),
    );
    assert_eq!(
        durable_state_signer_id(ed25519).bytes(),
        <[u8; 32]>::from(expected_legacy)
    );
    assert_eq!(
        durable_state_signer_id_for_key(DurableStatePublicKey::ed25519(ed25519)),
        durable_state_signer_id(ed25519)
    );

    let mut compressed = [0x61; 33];
    compressed[0] = 0x03;
    let key = DurableStatePublicKey::ecdsa_p256(compressed).unwrap();
    let expected_p256 = Sha256::digest(
        [
            b"AGENT-KERNEL-DURABLE-STATE-SIGNER-V2\0".as_slice(),
            &2u16.to_le_bytes(),
            compressed.as_slice(),
        ]
        .concat(),
    );
    assert_eq!(
        durable_state_signer_id_for_key(key).bytes(),
        <[u8; 32]>::from(expected_p256)
    );
}

#[test]
fn signer_record_and_manifest_bind_the_same_p256_algorithm() {
    let mut compressed = [0x71; 33];
    compressed[0] = 0x02;
    let key = DurableStatePublicKey::ecdsa_p256(compressed).unwrap();
    let signer = DurableStateSignerRecord::new_with_key(
        ResourceId::new(3),
        key,
        DurableStateSignerStatus::Active,
        7,
    )
    .unwrap();
    let manifest = algorithm_bound_manifest(signer.signer_id);

    assert_eq!(signer.public_key, key);
    assert_eq!(
        signer.signature_algorithm(),
        DurableSignatureAlgorithm::EcdsaP256Sha256
    );
    assert_eq!(
        manifest.version(),
        DurableArchiveManifestVersion::AlgorithmBound
    );
    assert_eq!(
        manifest.signature_algorithm(),
        DurableSignatureAlgorithm::EcdsaP256Sha256
    );
}

fn algorithm_bound_manifest(
    signer_id: agent_kernel_core::DurableStateSignerId,
) -> DurableArchiveManifest {
    let mut core = TestCore::new();
    core.register_agent(AgentId::new(1)).unwrap();
    let proposal = EventArchiveProposal::from_segment(None, core.events()).unwrap();
    DurableArchiveManifest::new_algorithm_bound(
        proposal,
        AgentId::new(1),
        CapabilityId::new(2),
        ResourceId::new(3),
        ResourceId::new(4),
        128,
        DurableStateDigest::from_archive(proposal.digest()),
        signer_id,
        DurableSignatureAlgorithm::EcdsaP256Sha256,
        7,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap()
}
