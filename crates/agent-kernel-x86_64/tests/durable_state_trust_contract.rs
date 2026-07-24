#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{
    durable_state_signer_id, durable_state_signer_id_for_key, DurableArchiveAnchor,
    DurableArchiveManifest, DurableArchiveManifestVersion, DurableSignatureAlgorithm,
    DurableStatePublicKey, DurableStateSignerRecord, DurableStateSignerStatus, ResourceId,
};
use agent_kernel_x86_64::durable_state::{DurableStateTrustPolicy, DurableStateVerificationError};

use durable_state_support::{
    manifest, p256_manifest, p256_signature, p256_signer_record, p256_signing_key, signature,
    signer_record, signing_key, POLICY_GENERATION, ROOT, STORAGE,
};

#[test]
fn active_root_scoped_state_signer_verifies_the_manifest() {
    let signing_key = signing_key(0x21);
    let manifest = manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signature = signature(&signing_key, manifest);
    let signer = signer_record(
        &signing_key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let policy = DurableStateTrustPolicy::new(core::slice::from_ref(&signer), POLICY_GENERATION);

    let verified = policy.verify(manifest, signature).unwrap();

    assert_eq!(verified.manifest(), manifest);
    assert_eq!(verified.signer_id(), signer.signer_id);
    assert_eq!(
        verified.manifest_digest(),
        agent_kernel_x86_64::durable_state::durable_archive_manifest_digest(manifest)
    );
}

#[test]
fn algorithm_bound_ed25519_state_signer_verifies_the_manifest() {
    let signing_key = signing_key(0x29);
    let legacy_manifest = manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let manifest = DurableArchiveManifest::from_algorithm_bound_fields(
        legacy_manifest.fields(),
        DurableSignatureAlgorithm::Ed25519,
    )
    .unwrap();
    let signer = signer_record(
        &signing_key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );

    let verified = DurableStateTrustPolicy::new(&[signer], POLICY_GENERATION)
        .verify(manifest, signature(&signing_key, manifest))
        .unwrap();

    assert_eq!(
        verified.manifest().version(),
        DurableArchiveManifestVersion::AlgorithmBound
    );
    assert_eq!(
        verified.manifest().signature_algorithm(),
        DurableSignatureAlgorithm::Ed25519
    );
}

#[test]
fn active_p256_state_signer_verifies_one_low_s_signature() {
    let signing_key = p256_signing_key(0x26);
    let manifest = p256_manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signature = p256_signature(&signing_key, manifest);
    let signer = p256_signer_record(
        &signing_key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );

    let verified = DurableStateTrustPolicy::new(&[signer], POLICY_GENERATION)
        .verify(manifest, signature)
        .unwrap();

    assert_eq!(verified.manifest(), manifest);
    assert_eq!(verified.signer_id(), signer.signer_id);
}

#[test]
fn p256_policy_rejects_algorithm_mismatch_and_high_s_signature() {
    let p256_key = p256_signing_key(0x27);
    let p256_manifest = p256_manifest(
        &p256_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let p256_signer = p256_signer_record(
        &p256_key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let high_s = high_s_signature(p256_signature(&p256_key, p256_manifest));
    assert_eq!(
        DurableStateTrustPolicy::new(&[p256_signer], POLICY_GENERATION)
            .verify(p256_manifest, high_s),
        Err(DurableStateVerificationError::SignatureNonCanonical)
    );

    let ed25519_key = signing_key(0x28);
    let ed25519_signer = signer_record(
        &ed25519_key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let mismatched_manifest = algorithm_mismatched_manifest(
        manifest(
            &ed25519_key,
            ROOT,
            STORAGE,
            POLICY_GENERATION,
            DurableArchiveAnchor::unanchored(),
        ),
        DurableSignatureAlgorithm::EcdsaP256Sha256,
    );
    assert_eq!(
        DurableStateTrustPolicy::new(&[ed25519_signer], POLICY_GENERATION).verify(
            mismatched_manifest,
            signature(&ed25519_key, mismatched_manifest)
        ),
        Err(DurableStateVerificationError::SignatureAlgorithmMismatch)
    );
}

#[test]
fn p256_policy_rejects_a_sec1_shape_with_an_invalid_curve_point() {
    let valid_key = p256_signing_key(0x2a);
    let valid_manifest = p256_manifest(
        &valid_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let mut malformed_bytes = [0; 33];
    malformed_bytes[0] = 0x02;
    malformed_bytes[32] = 0x01;
    let malformed_key = DurableStatePublicKey::EcdsaP256(malformed_bytes);
    let signer_id = durable_state_signer_id_for_key(malformed_key);
    let signer = DurableStateSignerRecord {
        signer_id,
        root: ROOT,
        public_key: malformed_key,
        status: DurableStateSignerStatus::Active,
        generation: POLICY_GENERATION,
    };
    let mut fields = valid_manifest.fields();
    fields.signer_id = signer_id;
    let manifest = DurableArchiveManifest::from_algorithm_bound_fields(
        fields,
        DurableSignatureAlgorithm::EcdsaP256Sha256,
    )
    .unwrap();

    assert_eq!(
        DurableStateTrustPolicy::new(&[signer], POLICY_GENERATION)
            .verify(manifest, p256_signature(&valid_key, manifest)),
        Err(DurableStateVerificationError::SignerKeyInvalid)
    );
}

#[test]
fn any_manifest_edit_invalidates_the_signature() {
    let signing_key = signing_key(0x22);
    let original = manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let changed = manifest(
        &signing_key,
        ROOT,
        ResourceId::new(99),
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signer = signer_record(
        &signing_key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let policy = DurableStateTrustPolicy::new(core::slice::from_ref(&signer), POLICY_GENERATION);

    assert_eq!(
        policy.verify(changed, signature(&signing_key, original)),
        Err(DurableStateVerificationError::SignatureInvalid)
    );
}

#[test]
fn unknown_and_duplicate_state_signers_are_rejected() {
    let signing_key = signing_key(0x23);
    let manifest = manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signature = signature(&signing_key, manifest);
    let signer = signer_record(
        &signing_key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );

    assert_eq!(
        DurableStateTrustPolicy::new(&[], POLICY_GENERATION).verify(manifest, signature),
        Err(DurableStateVerificationError::SignerNotTrusted)
    );
    assert_eq!(
        DurableStateTrustPolicy::new(&[signer, signer], POLICY_GENERATION)
            .verify(manifest, signature),
        Err(DurableStateVerificationError::TrustPolicyAmbiguous)
    );
}

#[test]
fn root_generation_revocation_and_key_identity_are_enforced_before_crypto() {
    let signing_key = signing_key(0x24);
    let manifest = manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signature = signature(&signing_key, manifest);
    let active = signer_record(
        &signing_key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );

    let wrong_root = DurableStateSignerRecord {
        root: ResourceId::new(88),
        ..active
    };
    assert_eq!(
        DurableStateTrustPolicy::new(&[wrong_root], POLICY_GENERATION).verify(manifest, signature),
        Err(DurableStateVerificationError::SignerRootMismatch)
    );

    assert_eq!(
        DurableStateTrustPolicy::new(&[active], POLICY_GENERATION + 1).verify(manifest, signature),
        Err(DurableStateVerificationError::PolicyGenerationMismatch)
    );

    let revoked = DurableStateSignerRecord {
        status: DurableStateSignerStatus::Revoked,
        ..active
    };
    assert_eq!(
        DurableStateTrustPolicy::new(&[revoked], POLICY_GENERATION).verify(manifest, signature),
        Err(DurableStateVerificationError::SignerRevoked)
    );

    let other_key = durable_state_support::signing_key(0x25)
        .verifying_key()
        .to_bytes();
    let mismatched_key = DurableStateSignerRecord {
        signer_id: durable_state_signer_id(signing_key.verifying_key().to_bytes()),
        public_key: DurableStatePublicKey::ed25519(other_key),
        ..active
    };
    assert_eq!(
        DurableStateTrustPolicy::new(&[mismatched_key], POLICY_GENERATION)
            .verify(manifest, signature),
        Err(DurableStateVerificationError::SignerKeyIdMismatch)
    );
}

fn algorithm_mismatched_manifest(
    manifest: DurableArchiveManifest,
    algorithm: DurableSignatureAlgorithm,
) -> DurableArchiveManifest {
    DurableArchiveManifest::from_algorithm_bound_fields(manifest.fields(), algorithm).unwrap()
}

fn high_s_signature(
    signature: agent_kernel_core::DurableArchiveSignature,
) -> agent_kernel_core::DurableArchiveSignature {
    const ORDER: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2, 0xfc, 0x63,
        0x25, 0x51,
    ];
    let mut bytes = signature.bytes();
    let mut borrow = 0u16;
    for index in (0..32).rev() {
        let minuend = u16::from(ORDER[index]);
        let subtrahend = u16::from(bytes[32 + index]) + borrow;
        if minuend >= subtrahend {
            bytes[32 + index] = (minuend - subtrahend) as u8;
            borrow = 0;
        } else {
            bytes[32 + index] = (minuend + 256 - subtrahend) as u8;
            borrow = 1;
        }
    }
    agent_kernel_core::DurableArchiveSignature::new(bytes)
}
