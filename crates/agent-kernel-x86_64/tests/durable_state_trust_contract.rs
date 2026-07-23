#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{
    durable_state_signer_id, DurableArchiveAnchor, DurableStateSignerRecord,
    DurableStateSignerStatus, ResourceId,
};
use agent_kernel_x86_64::durable_state::{DurableStateTrustPolicy, DurableStateVerificationError};

use durable_state_support::{
    manifest, signature, signer_record, signing_key, POLICY_GENERATION, ROOT, STORAGE,
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
        public_key: other_key,
        ..active
    };
    assert_eq!(
        DurableStateTrustPolicy::new(&[mismatched_key], POLICY_GENERATION)
            .verify(manifest, signature),
        Err(DurableStateVerificationError::SignerKeyIdMismatch)
    );
}
