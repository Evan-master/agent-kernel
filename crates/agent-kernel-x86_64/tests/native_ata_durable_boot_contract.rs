#[allow(dead_code)]
mod ata_block_support;
#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{
    durable_state_signer_id_for_key, DurableArchiveAnchor, DurableStatePublicKey,
    DurableStateSignerRecord, DurableStateSignerStatus, ResourceId,
};
use agent_kernel_hal::DURABLE_SLOT_BYTES;
use agent_kernel_x86_64::{
    ata::{
        AtaBlockDevice, AtaDeviceIdentity, AtaDrive, AtaDurableBinding, AtaDurableHead,
        AtaDurableStateBackend, AtaPioConfig, NativeAtaDurableBootState, NativeAtaDurableConfig,
        NativeAtaDurableConfigError, NativeAtaDurableInitError, ATA_DURABLE_SLOT_SECTORS,
        ATA_SECTOR_BYTES,
    },
    durable_state::{
        commit_durable_archive, DurableArchiveCapsuleError, DurableArchiveRecoveryError,
        DurableStateTrustPolicy,
    },
    native_durable_boot::NativeDurableStorageProfile,
};

use ata_block_support::{SectorDevice, SectorOperation};
use durable_state_support::{
    payload_and_manifest, signature, signer_record, signing_key, POLICY_GENERATION, ROOT, STORAGE,
};

const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;

#[test]
fn durable_profile_keeps_disabled_and_ata_modes_explicit() {
    let key = signing_key(0x90);
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let ata = config(signer);

    assert!(!NativeDurableStorageProfile::Disabled.is_enabled());
    assert_eq!(NativeDurableStorageProfile::Disabled.ata(), None);
    assert!(NativeDurableStorageProfile::Ata(ata).is_enabled());
    assert_eq!(NativeDurableStorageProfile::Ata(ata).ata(), Some(ata));
}

#[test]
fn empty_device_binds_genesis_without_writing_media() {
    let key = signing_key(0x91);
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let config = config(signer);
    let mut staging = Box::new([0; DURABLE_SLOT_BYTES]);
    let mut scratch = Box::new([0; DURABLE_SLOT_BYTES]);
    let mut payload = Box::new([0; agent_kernel_core::MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES]);

    let session = config
        .initialize_device(
            SectorDevice::new(identity()),
            staging.as_mut(),
            scratch.as_mut(),
            payload.as_mut(),
        )
        .expect("genesis session");

    assert_eq!(session.boot_state(), NativeAtaDurableBootState::Genesis);
    assert_eq!(session.backend().head(), Some(AtaDurableHead::Genesis));
    assert_eq!(session.backend().device().operations().len(), 256);
    assert!(session
        .backend()
        .device()
        .operations()
        .iter()
        .all(|operation| matches!(operation, SectorOperation::Read(_))));
    assert!(session.recovered_head().is_none());
    assert!(session.recovery_verifier().is_none());
}

#[test]
fn committed_device_yields_one_shot_recovered_boot_proof() {
    let key = signing_key(0x92);
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let policy = DurableStateTrustPolicy::new(core::slice::from_ref(&signer), POLICY_GENERATION);
    let (payload_bytes, manifest) = payload_and_manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let mut old_staging = Box::new([0; DURABLE_SLOT_BYTES]);
    let mut old_scratch = Box::new([0; DURABLE_SLOT_BYTES]);
    let binding = AtaDurableBinding::new(STORAGE, BASE_LBA, identity()).unwrap();
    let mut old_backend =
        AtaDurableStateBackend::new(SectorDevice::new(identity()), binding, old_staging.as_mut())
            .unwrap();
    old_backend.bind_head(AtaDurableHead::Genesis).unwrap();
    commit_durable_archive(
        &mut old_backend,
        policy,
        &payload_bytes,
        manifest,
        signature(&key, manifest),
        old_scratch.as_mut(),
    )
    .unwrap();
    let mut device = old_backend.into_device();
    device.simulate_power_loss();

    let mut staging = Box::new([0; DURABLE_SLOT_BYTES]);
    let mut scratch = Box::new([0; DURABLE_SLOT_BYTES]);
    let mut payload = Box::new([0; agent_kernel_core::MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES]);
    let mut session = config(signer)
        .initialize_device(device, staging.as_mut(), scratch.as_mut(), payload.as_mut())
        .expect("recovered session");
    let head = session.recovered_head().expect("recovered head");

    assert_eq!(
        session.boot_state(),
        NativeAtaDurableBootState::Recovered(head.generation())
    );
    assert_eq!(
        session.backend().head(),
        Some(AtaDurableHead::Recovered(head.generation()))
    );
    type RecoveredBoot = BootedKernel<2, 2, 4, 16, 4, 4, 0, 0, 0, 0>;
    let verifier = session.recovery_verifier_mut().expect("recovery verifier");
    let booted = RecoveredBoot::boot_recovered(BootConfig::default(), head, verifier).unwrap();
    assert!(verifier.is_consumed());
    assert_eq!(
        booted.kernel().events()[0].sequence,
        head.through_sequence() + 1
    );
}

#[test]
fn corrupt_committed_marker_stops_initialization() {
    let key = signing_key(0x93);
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let mut device = SectorDevice::new(identity());
    let mut sector = [0; ATA_SECTOR_BYTES];
    sector[ATA_SECTOR_BYTES - 1] = 1;
    device
        .write_sector(BASE_LBA + ATA_DURABLE_SLOT_SECTORS - 1, &sector)
        .unwrap();
    device.flush_cache().unwrap();
    let mut staging = Box::new([0; DURABLE_SLOT_BYTES]);
    let mut scratch = Box::new([0; DURABLE_SLOT_BYTES]);
    let mut payload = Box::new([0; agent_kernel_core::MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES]);

    let result = config(signer).initialize_device(
        device,
        staging.as_mut(),
        scratch.as_mut(),
        payload.as_mut(),
    );

    assert!(matches!(
        result,
        Err(NativeAtaDurableInitError::Recovery(
            DurableArchiveRecoveryError::Capsule {
                error: DurableArchiveCapsuleError::HeaderMagicMismatch,
                ..
            }
        ))
    ));
}

#[test]
fn config_rejects_aliased_resources_and_invalid_signer_scope() {
    let key = signing_key(0x94);
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    assert_eq!(
        NativeAtaDurableConfig::new(pio(), ROOT, ROOT, BASE_LBA, signer, POLICY_GENERATION,),
        Err(NativeAtaDurableConfigError::AliasedRootAndStorage)
    );
    let wrong_root = DurableStateSignerRecord {
        root: ResourceId::new(ROOT.raw() + 1),
        ..signer
    };
    assert_eq!(
        NativeAtaDurableConfig::new(
            pio(),
            ROOT,
            STORAGE,
            BASE_LBA,
            wrong_root,
            POLICY_GENERATION,
        ),
        Err(NativeAtaDurableConfigError::SignerRootMismatch)
    );
}

#[test]
fn config_rejects_malformed_signer_key_encoding() {
    let malformed_key = DurableStatePublicKey::EcdsaP256([0; 33]);
    let malformed_signer = DurableStateSignerRecord {
        signer_id: durable_state_signer_id_for_key(malformed_key),
        root: ROOT,
        public_key: malformed_key,
        status: DurableStateSignerStatus::Active,
        generation: POLICY_GENERATION,
    };
    assert_eq!(
        NativeAtaDurableConfig::new(
            pio(),
            ROOT,
            STORAGE,
            BASE_LBA,
            malformed_signer,
            POLICY_GENERATION,
        ),
        Err(NativeAtaDurableConfigError::SignerKeyEncodingInvalid)
    );
}

fn config(signer: DurableStateSignerRecord) -> NativeAtaDurableConfig {
    NativeAtaDurableConfig::new(pio(), ROOT, STORAGE, BASE_LBA, signer, POLICY_GENERATION).unwrap()
}

fn pio() -> AtaPioConfig {
    AtaPioConfig::new(0x170, 0x376, AtaDrive::Master, 10_000).unwrap()
}

fn identity() -> AtaDeviceIdentity {
    AtaDeviceIdentity::new(DEVICE_SECTORS).unwrap()
}
