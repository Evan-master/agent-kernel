mod ata_block_support;
#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{
    DurableRecoveryError, DurableRecoveryGuarantee, DurableSlot, DurableStateSignerStatus,
};
use agent_kernel_hal::{DurableStateBackendError, DURABLE_SLOT_BYTES};
use agent_kernel_x86_64::{
    ata::{
        AtaDeviceIdentity, AtaDurableBinding, AtaDurableHead, AtaDurableStateBackend, AtaPioError,
    },
    durable_state::{
        commit_durable_archive, recover_durable_archive, DurableArchiveCommitError,
        DurableArchiveRecoveryError, DurableStateTrustPolicy,
    },
};

use ata_block_support::SectorDevice;
use durable_state_support::{
    payload_and_manifest, signature, signer_record, signing_key, POLICY_GENERATION, ROOT, STORAGE,
};

const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;

fn identity() -> AtaDeviceIdentity {
    AtaDeviceIdentity::new(DEVICE_SECTORS).expect("identity")
}

fn binding() -> AtaDurableBinding {
    AtaDurableBinding::new(STORAGE, BASE_LBA, identity()).expect("binding")
}

#[test]
fn signed_transaction_survives_a_cold_backend_reconstruction() {
    let key = signing_key(0x71);
    let (payload, manifest) = payload_and_manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        agent_kernel_core::DurableArchiveAnchor::unanchored(),
    );
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let policy = DurableStateTrustPolicy::new(core::slice::from_ref(&signer), POLICY_GENERATION);
    let mut staging = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let mut scratch = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let mut backend =
        AtaDurableStateBackend::new(SectorDevice::new(identity()), binding(), &mut staging)
            .expect("backend");
    backend.bind_head(AtaDurableHead::Genesis).unwrap();

    let commit = commit_durable_archive(
        &mut backend,
        policy,
        &payload,
        manifest,
        signature(&key, manifest),
        scratch.as_mut(),
    )
    .expect("native transaction");

    assert_eq!(commit.receipt().slot(), DurableSlot::A);
    assert_eq!(commit.receipt().flush_epoch(), 3);
    assert_eq!(backend.head(), Some(AtaDurableHead::Recovered(1)));
    assert_eq!(backend.device().operations().len(), 390);

    let mut device = backend.into_device();
    device.simulate_power_loss();
    let mut cold_staging = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let mut cold_scratch = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let mut cold =
        AtaDurableStateBackend::new(device, binding(), &mut cold_staging).expect("cold backend");
    let recovered = recover_durable_archive(&mut cold, policy, STORAGE, cold_scratch.as_mut())
        .expect("signed cold recovery");

    assert_eq!(recovered.generation(), 1);
    assert_eq!(recovered.slot(), DurableSlot::A);
    assert_eq!(
        recovered.guarantee(),
        DurableRecoveryGuarantee::RollbackEvident
    );
    assert_eq!(recovered.receipt().flush_epoch(), 1);
    cold.bind_head(AtaDurableHead::Recovered(recovered.generation()))
        .unwrap();
}

#[test]
fn power_loss_before_and_after_footer_flush_has_deterministic_recovery() {
    for (failed_operation, expected_generation) in [(10, None), (262, None), (263, Some(1))] {
        let key = signing_key(0x72);
        let (payload, manifest) = payload_and_manifest(
            &key,
            ROOT,
            STORAGE,
            POLICY_GENERATION,
            agent_kernel_core::DurableArchiveAnchor::unanchored(),
        );
        let signer = signer_record(
            &key,
            ROOT,
            DurableStateSignerStatus::Active,
            POLICY_GENERATION,
        );
        let policy =
            DurableStateTrustPolicy::new(core::slice::from_ref(&signer), POLICY_GENERATION);
        let device =
            SectorDevice::failing_at(identity(), failed_operation, AtaPioError::BusyTimeout);
        let mut staging = Box::new([0_u8; DURABLE_SLOT_BYTES]);
        let mut scratch = Box::new([0_u8; DURABLE_SLOT_BYTES]);
        let mut backend =
            AtaDurableStateBackend::new(device, binding(), &mut staging).expect("backend");
        backend.bind_head(AtaDurableHead::Genesis).unwrap();

        assert_eq!(
            commit_durable_archive(
                &mut backend,
                policy,
                &payload,
                manifest,
                signature(&key, manifest),
                scratch.as_mut(),
            ),
            Err(DurableArchiveCommitError::Backend(
                DurableStateBackendError::Interrupted
            ))
        );

        let mut device = backend.into_device();
        device.simulate_power_loss();
        let mut cold_staging = Box::new([0_u8; DURABLE_SLOT_BYTES]);
        let mut cold_scratch = Box::new([0_u8; DURABLE_SLOT_BYTES]);
        let mut cold = AtaDurableStateBackend::new(device, binding(), &mut cold_staging)
            .expect("cold backend");
        let recovered = recover_durable_archive(&mut cold, policy, STORAGE, cold_scratch.as_mut());

        match expected_generation {
            Some(generation) => {
                assert_eq!(recovered.unwrap().generation(), generation);
            }
            None => assert_eq!(
                recovered,
                Err(DurableArchiveRecoveryError::Selection(
                    DurableRecoveryError::NoCommittedSlot
                ))
            ),
        }
    }
}
