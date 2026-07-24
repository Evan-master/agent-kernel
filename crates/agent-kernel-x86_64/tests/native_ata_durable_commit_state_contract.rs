#[allow(dead_code)]
mod ata_block_support;
mod durable_archive_kernel_support;
#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{DurableStateSignerStatus, KernelError};
use agent_kernel_hal::DURABLE_SLOT_BYTES;
use agent_kernel_x86_64::{
    ata::{
        AtaDeviceIdentity, AtaDrive, AtaDurableHead, AtaPioConfig, AtaPioError,
        NativeAtaDurableCommitError, NativeAtaDurableConfig, NativeAtaDurablePrepareError,
        NativeDurableArchiveCaller,
    },
    durable_archive_request::{
        DURABLE_ARCHIVE_REQUEST_BYTES, DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET,
        DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET,
    },
    durable_state::DurableStateVerificationError,
};

use ata_block_support::SectorDevice;
use durable_archive_kernel_support::{launched_archive_kernel, ArchiveKernelFixture};
use durable_state_support::{signature, signer_record, signing_key, POLICY_GENERATION};

const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;
const CALL_DATA_GENERATION: u64 = 11;

#[test]
fn signed_preparation_commits_then_releases_core_events_once() {
    let key = signing_key(0xc1);
    let mut fixture = launched_archive_kernel();
    let (proposal, preflight, caller) = archive_contract(&fixture);
    let retained = fixture.kernel.events()[proposal.count()..].to_vec();
    let mut buffers = buffers();
    let mut session = config(
        &fixture,
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
    let preparation = session
        .prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION,
        )
        .unwrap();
    let request = signed_request(&key, preparation);

    let mut verified = session
        .commit_prepared(caller, preflight, &request)
        .expect("ATA commit");
    let receipt = verified.receipt();
    let checkpoint = fixture
        .kernel
        .commit_verified_event_archive(
            fixture.actor,
            fixture.archive_authority,
            fixture.storage_authority,
            proposal,
            receipt,
            &mut verified,
        )
        .expect("Core release");

    assert!(verified.is_consumed());
    assert_eq!(checkpoint.proposal(), proposal);
    assert_eq!(fixture.kernel.events(), retained.as_slice());
    assert_eq!(session.preparation(), None);
    assert!(!session.is_faulted());
    assert_eq!(
        session.backend().head(),
        Some(AtaDurableHead::Recovered(proposal.generation()))
    );
}

#[test]
fn invalid_signature_keeps_preparation_retryable_with_zero_writes() {
    let key = signing_key(0xc2);
    let fixture = launched_archive_kernel();
    let (proposal, preflight, caller) = archive_contract(&fixture);
    let mut buffers = buffers();
    let mut session = config(
        &fixture,
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
    let preparation = session
        .prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION,
        )
        .unwrap();
    let baseline = session.backend().device().operations().len();
    let request = preparation.request_bytes();

    assert_eq!(
        session.commit_prepared(caller, preflight, &request),
        Err(NativeAtaDurableCommitError::Trust(
            DurableStateVerificationError::SignatureInvalid
        ))
    );
    assert_eq!(session.preparation(), Some(preparation));
    assert!(!session.is_faulted());
    assert_eq!(session.backend().device().operations().len(), baseline);
}

#[test]
fn transaction_failure_faults_session_until_media_is_rescanned() {
    let key = signing_key(0xc3);
    let fixture = launched_archive_kernel();
    let (proposal, preflight, caller) = archive_contract(&fixture);
    let mut buffers = buffers();
    let mut session = config(
        &fixture,
        signer_record(
            &key,
            fixture.root,
            DurableStateSignerStatus::Active,
            POLICY_GENERATION,
        ),
    )
    .initialize_device(
        SectorDevice::failing_at(identity(), 257, AtaPioError::BusyTimeout),
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
    let request = signed_request(&key, preparation);

    assert!(matches!(
        session.commit_prepared(caller, preflight, &request),
        Err(NativeAtaDurableCommitError::Transaction(_))
    ));
    assert!(session.is_faulted());
    assert_eq!(session.preparation(), None);
    assert_eq!(
        session.prepare(
            caller,
            preflight,
            &fixture.kernel.events()[..proposal.count()],
            CALL_DATA_GENERATION + 1,
        ),
        Err(NativeAtaDurablePrepareError::SessionFaulted)
    );
    assert_eq!(
        fixture.kernel.event_archive_checkpoint(),
        None,
        "Core retains live Events after storage failure"
    );
    assert_ne!(
        fixture.kernel.preflight_durable_event_archive(
            fixture.actor,
            fixture.archive_authority,
            fixture.storage_authority,
            fixture.storage,
            proposal,
        ),
        Err(KernelError::EventArchiveProposalMismatch)
    );
}

fn archive_contract(
    fixture: &ArchiveKernelFixture,
) -> (
    agent_kernel_core::EventArchiveProposal,
    agent_kernel_core::DurableArchivePreflight,
    NativeDurableArchiveCaller,
) {
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
    (proposal, preflight, caller)
}

fn signed_request(
    key: &ed25519_dalek::SigningKey,
    preparation: agent_kernel_x86_64::ata::NativeDurableArchivePreparation,
) -> [u8; DURABLE_ARCHIVE_REQUEST_BYTES] {
    let mut bytes = preparation.request_bytes();
    bytes[DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET..DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET]
        .copy_from_slice(&signature(key, preparation.manifest()).bytes());
    bytes
}

fn config(
    fixture: &ArchiveKernelFixture,
    signer: agent_kernel_core::DurableStateSignerRecord,
) -> NativeAtaDurableConfig {
    NativeAtaDurableConfig::new(
        AtaPioConfig::new(0x170, 0x376, AtaDrive::Master, 10_000).unwrap(),
        fixture.root,
        fixture.storage,
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
