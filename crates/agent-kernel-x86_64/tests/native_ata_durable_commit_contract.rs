#[allow(dead_code)]
mod ata_block_support;
#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{
    AgentId, CapabilityId, DurableArchiveAnchor, DurableStateSignerStatus, EventArchiveProposal,
    KernelCore,
};
use agent_kernel_hal::DURABLE_SLOT_BYTES;
use agent_kernel_x86_64::{
    ata::{
        AtaDeviceIdentity, AtaDrive, AtaDurableHead, AtaPioConfig, NativeAtaDurableCommitError,
        NativeAtaDurableConfig,
    },
    durable_archive_request::{
        DurableArchiveRequest, DURABLE_ARCHIVE_REQUEST_BYTES,
        DURABLE_ARCHIVE_REQUEST_FORMAT_VERSION, DURABLE_ARCHIVE_REQUEST_MAGIC,
    },
    durable_state::{
        encode_durable_archive_manifest, DurableArchiveCommitError, DurableStateVerificationError,
    },
};

use ata_block_support::SectorDevice;
use durable_state_support::{
    payload_and_manifest, signature, signer_record, signing_key, POLICY_GENERATION, ROOT, STORAGE,
};

type ArchiveCore = KernelCore<1, 0, 0, 4, 0, 0, 0, 0, 0, 0>;

const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;
const REQUEST_GENERATION: u64 = 1;
const STORAGE_AUTHORITY: CapabilityId = CapabilityId::new(7);

#[test]
fn signed_request_commits_through_the_boot_owned_ata_session() {
    let key = signing_key(0xa1);
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let mut buffers = buffers();
    let mut session = config(signer)
        .initialize_device(
            SectorDevice::new(identity()),
            buffers.0.as_mut(),
            buffers.1.as_mut(),
            buffers.2.as_mut(),
        )
        .unwrap();
    let (core, proposal) = archive_core();
    let (_, manifest) = payload_and_manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let request = request(&key, manifest);

    let commit = session
        .commit(
            AgentId::new(1),
            CapabilityId::new(2),
            proposal,
            core.events(),
            request,
        )
        .expect("native durable commit");

    assert_eq!(request.storage_authority(), STORAGE_AUTHORITY);
    assert_eq!(commit.manifest(), manifest);
    assert_eq!(commit.receipt().generation(), 1);
    assert_eq!(session.backend().head(), Some(AtaDurableHead::Recovered(1)));
    assert_eq!(session.backend().device().operations().len(), 256 + 390);
}

#[test]
fn manifest_identity_mismatch_causes_zero_device_writes() {
    let key = signing_key(0xa2);
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let mut buffers = buffers();
    let mut session = config(signer)
        .initialize_device(
            SectorDevice::new(identity()),
            buffers.0.as_mut(),
            buffers.1.as_mut(),
            buffers.2.as_mut(),
        )
        .unwrap();
    let baseline = session.backend().device().operations().len();
    let (core, proposal) = archive_core();
    let (_, manifest) = payload_and_manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );

    assert_eq!(
        session.commit(
            AgentId::new(2),
            CapabilityId::new(2),
            proposal,
            core.events(),
            request(&key, manifest),
        ),
        Err(NativeAtaDurableCommitError::ManifestMismatch)
    );
    assert_eq!(session.backend().device().operations().len(), baseline);
    assert_eq!(session.backend().head(), Some(AtaDurableHead::Genesis));
}

#[test]
fn invalid_signature_is_rejected_before_the_first_write() {
    let key = signing_key(0xa3);
    let signer = signer_record(
        &key,
        ROOT,
        DurableStateSignerStatus::Active,
        POLICY_GENERATION,
    );
    let mut buffers = buffers();
    let mut session = config(signer)
        .initialize_device(
            SectorDevice::new(identity()),
            buffers.0.as_mut(),
            buffers.1.as_mut(),
            buffers.2.as_mut(),
        )
        .unwrap();
    let baseline = session.backend().device().operations().len();
    let (core, proposal) = archive_core();
    let (_, manifest) = payload_and_manifest(
        &key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let mut bytes = request_bytes(&key, manifest);
    bytes[317] ^= 0x80;
    let request = DurableArchiveRequest::decode(&bytes, REQUEST_GENERATION).unwrap();

    assert_eq!(
        session.commit(
            AgentId::new(1),
            CapabilityId::new(2),
            proposal,
            core.events(),
            request,
        ),
        Err(NativeAtaDurableCommitError::Transaction(
            DurableArchiveCommitError::Trust(DurableStateVerificationError::SignatureInvalid)
        ))
    );
    assert_eq!(session.backend().device().operations().len(), baseline);
}

fn archive_core() -> (ArchiveCore, EventArchiveProposal) {
    let mut core = ArchiveCore::new();
    core.register_agent(AgentId::new(1)).unwrap();
    let proposal = EventArchiveProposal::from_segment(None, core.events()).unwrap();
    (core, proposal)
}

fn request(
    key: &ed25519_dalek::SigningKey,
    manifest: agent_kernel_core::DurableArchiveManifest,
) -> DurableArchiveRequest {
    DurableArchiveRequest::decode(&request_bytes(key, manifest), REQUEST_GENERATION).unwrap()
}

fn request_bytes(
    key: &ed25519_dalek::SigningKey,
    manifest: agent_kernel_core::DurableArchiveManifest,
) -> [u8; DURABLE_ARCHIVE_REQUEST_BYTES] {
    let mut bytes = [0; DURABLE_ARCHIVE_REQUEST_BYTES];
    bytes[..8].copy_from_slice(&DURABLE_ARCHIVE_REQUEST_MAGIC);
    bytes[8..10].copy_from_slice(&DURABLE_ARCHIVE_REQUEST_FORMAT_VERSION.to_le_bytes());
    bytes[12..16].copy_from_slice(&(DURABLE_ARCHIVE_REQUEST_BYTES as u32).to_le_bytes());
    bytes[16..24].copy_from_slice(&REQUEST_GENERATION.to_le_bytes());
    bytes[24..32].copy_from_slice(&STORAGE_AUTHORITY.raw().to_le_bytes());
    bytes[32..317].copy_from_slice(&encode_durable_archive_manifest(manifest));
    bytes[317..381].copy_from_slice(&signature(key, manifest).bytes());
    bytes
}

fn config(signer: agent_kernel_core::DurableStateSignerRecord) -> NativeAtaDurableConfig {
    NativeAtaDurableConfig::new(
        AtaPioConfig::new(0x170, 0x376, AtaDrive::Master, 10_000).unwrap(),
        ROOT,
        STORAGE,
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
