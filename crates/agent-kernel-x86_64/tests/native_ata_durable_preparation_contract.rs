#[allow(dead_code)]
mod ata_block_support;
mod durable_archive_kernel_support;
#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{DurableStateSignerStatus, EventArchiveEncodingError};
use agent_kernel_hal::DURABLE_SLOT_BYTES;
use agent_kernel_x86_64::{
    ata::{
        AtaDeviceIdentity, AtaDrive, AtaPioConfig, NativeAtaDurableConfig,
        NativeAtaDurablePrepareError, NativeDurableArchiveCaller,
    },
    durable_archive_request::DurableArchiveRequest,
};

use ata_block_support::SectorDevice;
use durable_archive_kernel_support::launched_archive_kernel;
use durable_state_support::{signer_record, signing_key, POLICY_GENERATION};

const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;
const CALL_DATA_GENERATION: u64 = 7;

#[test]
fn preparation_binds_preflight_caller_payload_and_unsigned_request() {
    let key = signing_key(0xb1);
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
    let mut buffers = buffers();
    let mut session = config(
        fixture.root,
        fixture.storage,
        signer_record(
            &key,
            fixture.root,
            DurableStateSignerStatus::Active,
            POLICY_GENERATION,
        ),
    )
    .initialize_device(
        SectorDevice::new(identity()),
        buffers.0.as_mut(),
        buffers.1.as_mut(),
        buffers.2.as_mut(),
    )
    .unwrap();
    let baseline = session.backend().device().operations().len();

    let preparation = session
        .prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION,
        )
        .expect("native durable preparation");
    let request =
        DurableArchiveRequest::decode(&preparation.request_bytes(), CALL_DATA_GENERATION).unwrap();

    assert_eq!(preparation.caller(), caller);
    assert_eq!(preparation.preflight(), preflight);
    assert_eq!(preparation.call_data_generation(), CALL_DATA_GENERATION);
    assert_eq!(preparation.manifest(), request.manifest());
    assert_eq!(request.storage_authority(), fixture.storage_authority);
    assert_eq!(request.signature().bytes(), [0; 64]);
    assert_eq!(session.preparation(), Some(preparation));
    assert_eq!(session.backend().device().operations().len(), baseline);
    assert_eq!(
        session.prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION + 1,
        ),
        Err(NativeAtaDurablePrepareError::AlreadyPrepared)
    );
    assert_eq!(session.backend().device().operations().len(), baseline);
}

#[test]
fn preparation_rejects_caller_and_event_mismatch_before_device_io() {
    let key = signing_key(0xb2);
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
    let wrong_caller = NativeDurableArchiveCaller::new(
        agent_kernel_core::AgentId::new(fixture.actor.raw() + 1),
        fixture.task,
        fixture.image,
    )
    .unwrap();
    let mut buffers = buffers();
    let mut session = config(
        fixture.root,
        fixture.storage,
        signer_record(
            &key,
            fixture.root,
            DurableStateSignerStatus::Active,
            POLICY_GENERATION,
        ),
    )
    .initialize_device(
        SectorDevice::new(identity()),
        buffers.0.as_mut(),
        buffers.1.as_mut(),
        buffers.2.as_mut(),
    )
    .unwrap();
    let baseline = session.backend().device().operations().len();

    assert_eq!(
        session.prepare(
            wrong_caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION,
        ),
        Err(NativeAtaDurablePrepareError::CallerMismatch)
    );
    assert_eq!(
        session.prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count() - 1],
            CALL_DATA_GENERATION,
        ),
        Err(NativeAtaDurablePrepareError::Archive(
            EventArchiveEncodingError::ProposalMismatch
        ))
    );
    assert_eq!(session.preparation(), None);
    assert_eq!(session.backend().device().operations().len(), baseline);
}

fn config(
    root: agent_kernel_core::ResourceId,
    storage: agent_kernel_core::ResourceId,
    signer: agent_kernel_core::DurableStateSignerRecord,
) -> NativeAtaDurableConfig {
    NativeAtaDurableConfig::new(
        AtaPioConfig::new(0x170, 0x376, AtaDrive::Master, 10_000).unwrap(),
        root,
        storage,
        BASE_LBA,
        signer,
        POLICY_GENERATION,
    )
    .unwrap()
}

fn identity() -> AtaDeviceIdentity {
    AtaDeviceIdentity::new(DEVICE_SECTORS).unwrap()
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
