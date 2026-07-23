mod ata_block_support;

use agent_kernel_core::{DurableSlot, ResourceId};
use agent_kernel_hal::{
    DurableSlotRegion, DurableSlotTarget, DurableSlotWrite, DurableStateBackend,
    DurableStateBackendError, DURABLE_SLOT_BYTES, DURABLE_SLOT_FOOTER_BYTES,
    DURABLE_SLOT_HEADER_BYTES,
};
use agent_kernel_x86_64::ata::{
    AtaDeviceIdentity, AtaDurableBinding, AtaDurableBindingError, AtaDurableHead,
    AtaDurableStateBackend, AtaPioError, ATA_DURABLE_RANGE_SECTORS, ATA_DURABLE_SLOT_SECTORS,
};

use ata_block_support::{SectorDevice, SectorOperation};

const STORAGE: ResourceId = ResourceId::new(41);
const BASE_LBA: u64 = 256;
const DEVICE_SECTORS: u64 = 4096;

fn identity() -> AtaDeviceIdentity {
    AtaDeviceIdentity::new(DEVICE_SECTORS).expect("identity")
}

fn binding() -> AtaDurableBinding {
    AtaDurableBinding::new(STORAGE, BASE_LBA, identity()).expect("binding")
}

fn target(generation: u64) -> DurableSlotTarget {
    let slot = DurableSlot::for_generation(generation).expect("nonzero generation");
    DurableSlotTarget::new(STORAGE, slot, generation).expect("target")
}

fn write<'a>(
    target: DurableSlotTarget,
    region: DurableSlotRegion,
    bytes: &'a [u8],
) -> DurableSlotWrite<'a> {
    DurableSlotWrite::new(target, region, bytes).expect("semantic write")
}

#[test]
fn binding_freezes_aligned_dual_slot_range() {
    let binding = binding();
    assert_eq!(binding.slot_lba(DurableSlot::A), BASE_LBA);
    assert_eq!(
        binding.slot_lba(DurableSlot::B),
        BASE_LBA + ATA_DURABLE_SLOT_SECTORS
    );
    assert_eq!(binding.range_sectors(), ATA_DURABLE_RANGE_SECTORS);

    assert_eq!(
        AtaDurableBinding::new(ResourceId::new(0), BASE_LBA, identity()),
        Err(AtaDurableBindingError::ZeroStorageResource)
    );
    assert_eq!(
        AtaDurableBinding::new(STORAGE, BASE_LBA + 1, identity()),
        Err(AtaDurableBindingError::BaseLbaUnaligned {
            lba: BASE_LBA + 1,
            required_sectors: ATA_DURABLE_SLOT_SECTORS,
        })
    );
    let small = AtaDeviceIdentity::new(BASE_LBA + ATA_DURABLE_RANGE_SECTORS - 1).unwrap();
    assert_eq!(
        AtaDurableBinding::new(STORAGE, BASE_LBA, small),
        Err(AtaDurableBindingError::RangeExceedsDevice {
            required_exclusive: BASE_LBA + ATA_DURABLE_RANGE_SECTORS,
            sector_count: small.sector_count(),
        })
    );
}

#[test]
fn unbound_or_wrong_generation_writes_issue_no_device_command() {
    let mut staging = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let mut backend =
        AtaDurableStateBackend::new(SectorDevice::new(identity()), binding(), &mut staging)
            .expect("backend");
    let header = [0x11; DURABLE_SLOT_HEADER_BYTES];

    assert_eq!(
        backend.write(write(target(1), DurableSlotRegion::PreparedHeader, &header)),
        Err(DurableStateBackendError::PhaseViolation)
    );
    backend.bind_head(AtaDurableHead::Recovered(7)).unwrap();
    assert_eq!(
        backend.write(write(target(1), DurableSlotRegion::PreparedHeader, &header)),
        Err(DurableStateBackendError::ActiveGenerationConflict)
    );
    assert!(backend.device().operations().is_empty());
}

#[test]
fn semantic_regions_map_to_bounded_sector_writes_and_flushes() {
    let mut staging = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let mut backend =
        AtaDurableStateBackend::new(SectorDevice::new(identity()), binding(), &mut staging)
            .expect("backend");
    backend.bind_head(AtaDurableHead::Genesis).unwrap();
    let target = target(1);
    let header = [0x11; DURABLE_SLOT_HEADER_BYTES];
    let body = [0x22; 1024];
    let footer = [0x33; DURABLE_SLOT_FOOTER_BYTES];

    backend
        .write(write(target, DurableSlotRegion::PreparedHeader, &header))
        .unwrap();
    assert_eq!(
        backend.device().operations(),
        &[
            SectorOperation::Write(BASE_LBA),
            SectorOperation::Write(BASE_LBA + ATA_DURABLE_SLOT_SECTORS - 1),
        ]
    );
    assert_eq!(
        &backend.device().sector(BASE_LBA)[..DURABLE_SLOT_HEADER_BYTES],
        &header
    );
    assert_eq!(backend.flush(target).unwrap().epoch(), 1);

    backend
        .write(write(target, DurableSlotRegion::Body, &body))
        .unwrap();
    let operations = backend.device().operations();
    assert_eq!(operations[3], SectorOperation::Write(BASE_LBA));
    assert_eq!(
        operations[3 + ATA_DURABLE_SLOT_SECTORS as usize - 1],
        SectorOperation::Write(BASE_LBA + ATA_DURABLE_SLOT_SECTORS - 1)
    );
    assert_eq!(backend.flush(target).unwrap().epoch(), 2);

    let mut prepared = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let readback = backend
        .read_slot(STORAGE, DurableSlot::A, prepared.as_mut())
        .unwrap();
    assert_eq!(readback.bytes_read(), DURABLE_SLOT_BYTES);
    assert_eq!(readback.flush_epoch(), 2);
    assert_eq!(&prepared[..DURABLE_SLOT_HEADER_BYTES], &header);
    assert_eq!(
        &prepared[DURABLE_SLOT_HEADER_BYTES..DURABLE_SLOT_HEADER_BYTES + body.len()],
        &body
    );
    assert!(prepared[DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES..]
        .iter()
        .all(|byte| *byte == 0));

    backend
        .write(write(target, DurableSlotRegion::CommitFooter, &footer))
        .unwrap();
    assert_eq!(
        backend.device().operations().last(),
        Some(&SectorOperation::Write(
            BASE_LBA + ATA_DURABLE_SLOT_SECTORS - 1
        ))
    );
    assert_eq!(backend.flush(target).unwrap().epoch(), 3);
    assert_eq!(backend.head(), Some(AtaDurableHead::Recovered(1)));
    assert_eq!(
        &backend
            .device()
            .sector(BASE_LBA + ATA_DURABLE_SLOT_SECTORS - 1)[448..],
        &footer
    );
}

#[test]
fn interrupted_flush_preserves_epoch_head_and_dirty_phase_for_retry() {
    let mut staging = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let device = SectorDevice::failing_at(identity(), 3, AtaPioError::BusyTimeout);
    let mut backend =
        AtaDurableStateBackend::new(device, binding(), &mut staging).expect("backend");
    backend.bind_head(AtaDurableHead::Genesis).unwrap();
    let target = target(1);
    let header = [0x44; DURABLE_SLOT_HEADER_BYTES];

    backend
        .write(write(target, DurableSlotRegion::PreparedHeader, &header))
        .unwrap();
    assert_eq!(
        backend.flush(target),
        Err(DurableStateBackendError::Interrupted)
    );
    assert_eq!(backend.flush_epoch(), 0);
    assert_eq!(backend.head(), Some(AtaDurableHead::Genesis));
    assert_eq!(backend.last_device_error(), Some(AtaPioError::BusyTimeout));
    assert_eq!(backend.flush(target).unwrap().epoch(), 1);
}

#[test]
fn phase_and_buffer_failures_are_atomic() {
    let mut staging = Box::new([0_u8; DURABLE_SLOT_BYTES]);
    let mut backend =
        AtaDurableStateBackend::new(SectorDevice::new(identity()), binding(), &mut staging)
            .expect("backend");
    backend.bind_head(AtaDurableHead::Genesis).unwrap();
    let target = target(1);
    let body = [0x55; 32];

    assert_eq!(
        backend.write(write(target, DurableSlotRegion::Body, &body)),
        Err(DurableStateBackendError::PhaseViolation)
    );
    let mut short = [0_u8; 32];
    assert_eq!(
        backend.read_slot(STORAGE, DurableSlot::A, &mut short),
        Err(DurableStateBackendError::BufferTooSmall {
            required: DURABLE_SLOT_BYTES,
            available: short.len(),
        })
    );
    assert!(backend.device().operations().is_empty());
}
