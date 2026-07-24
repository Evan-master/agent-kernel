#[allow(dead_code)]
mod ata_block_support;
mod durable_archive_kernel_support;
#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::DurableStateSignerStatus;
use agent_kernel_hal::DURABLE_SLOT_BYTES;
use agent_kernel_x86_64::ata::{
    AtaDeviceIdentity, AtaDrive, AtaPioConfig, NativeAtaDurableCommitError, NativeAtaDurableConfig,
    NativeDurableArchiveCaller,
};

use ata_block_support::SectorDevice;
use durable_archive_kernel_support::launched_archive_kernel;
use durable_state_support::{signer_record, signing_key, POLICY_GENERATION};

const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;
const CALL_DATA_GENERATION: u64 = 17;

#[test]
fn signed_request_mutation_outside_signature_is_rejected_before_io() {
    let fixture = launched_archive_kernel();
    let proposal = fixture
        .kernel
        .sys_prepare_event_archive(fixture.kernel.events()[3].sequence)
        .unwrap();
    let preflight = fixture
        .kernel
        .preflight_durable_event_archive(
            fixture.actor,
            fixture.archive_authority,
            fixture.storage_authority,
            fixture.storage,
            proposal,
        )
        .unwrap();
    let caller =
        NativeDurableArchiveCaller::new(fixture.actor, fixture.task, fixture.image).unwrap();
    let key = signing_key(0xc4);
    let mut buffers = buffers();
    let mut session = NativeAtaDurableConfig::new(
        AtaPioConfig::new(0x170, 0x376, AtaDrive::Master, 10_000).unwrap(),
        fixture.root,
        fixture.storage,
        BASE_LBA,
        signer_record(
            &key,
            fixture.root,
            DurableStateSignerStatus::Active,
            POLICY_GENERATION,
        ),
        POLICY_GENERATION,
    )
    .unwrap()
    .initialize_device(
        SectorDevice::new(AtaDeviceIdentity::new(DEVICE_SECTORS).unwrap()),
        buffers.0.as_mut(),
        buffers.1.as_mut(),
        buffers.2.as_mut(),
    )
    .unwrap();
    let preparation = session
        .prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION,
        )
        .unwrap();
    let baseline = session.backend().device().operations().len();
    let mut changed = preparation.request_bytes();
    changed[24] ^= 1;

    assert_eq!(
        session.commit_prepared(caller, preflight, &changed),
        Err(NativeAtaDurableCommitError::RequestMismatch)
    );
    assert_eq!(session.preparation(), Some(preparation));
    assert!(!session.is_faulted());
    assert_eq!(session.backend().device().operations().len(), baseline);
}

type Buffers = (
    Box<[u8; DURABLE_SLOT_BYTES]>,
    Box<[u8; DURABLE_SLOT_BYTES]>,
    Box<[u8; agent_kernel_core::MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES]>,
);

fn buffers() -> Buffers {
    (
        Box::new([0; DURABLE_SLOT_BYTES]),
        Box::new([0; DURABLE_SLOT_BYTES]),
        Box::new([0; agent_kernel_core::MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES]),
    )
}
