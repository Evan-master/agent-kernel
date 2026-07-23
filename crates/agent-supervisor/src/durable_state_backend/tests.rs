use agent_kernel_core::{DurableSlot, ResourceId};
use agent_kernel_hal::{
    DurableSlotRegion, DurableSlotTarget, DurableSlotWrite, DurableStateBackend,
    DurableStateBackendError, DURABLE_SLOT_BYTES, DURABLE_SLOT_FOOTER_BYTES,
    DURABLE_SLOT_HEADER_BYTES,
};

use super::{InMemoryDurableSlotPhase, InMemoryDurableStateBackend};

const STORAGE: ResourceId = ResourceId::new(7);

fn target(generation: u64) -> DurableSlotTarget {
    DurableSlotTarget::new(
        STORAGE,
        DurableSlot::for_generation(generation).unwrap(),
        generation,
    )
    .unwrap()
}

fn drive_transaction(
    backend: &mut InMemoryDurableStateBackend,
    target: DurableSlotTarget,
) -> Result<(), DurableStateBackendError> {
    let header = [target.generation() as u8; DURABLE_SLOT_HEADER_BYTES];
    let body = [0x52; 32];
    let footer = [0xc3; DURABLE_SLOT_FOOTER_BYTES];
    let mut readback = vec![0; DURABLE_SLOT_BYTES];

    backend.write(
        DurableSlotWrite::new(target, DurableSlotRegion::PreparedHeader, &header).unwrap(),
    )?;
    backend.flush(target)?;
    backend.write(DurableSlotWrite::new(target, DurableSlotRegion::Body, &body).unwrap())?;
    backend.flush(target)?;
    backend.read_slot(target.storage(), target.slot(), &mut readback)?;
    backend
        .write(DurableSlotWrite::new(target, DurableSlotRegion::CommitFooter, &footer).unwrap())?;
    backend.flush(target)?;
    backend.read_slot(target.storage(), target.slot(), &mut readback)?;
    Ok(())
}

#[test]
fn transaction_phases_require_a_flush_before_the_next_region() {
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    let target = target(1);
    let header = [0x11; DURABLE_SLOT_HEADER_BYTES];
    let body = [0x22; 8];

    assert_eq!(
        backend.write(DurableSlotWrite::new(target, DurableSlotRegion::Body, &body).unwrap()),
        Err(DurableStateBackendError::PhaseViolation)
    );
    backend
        .write(DurableSlotWrite::new(target, DurableSlotRegion::PreparedHeader, &header).unwrap())
        .unwrap();
    assert_eq!(
        backend.write(DurableSlotWrite::new(target, DurableSlotRegion::Body, &body).unwrap()),
        Err(DurableStateBackendError::PhaseViolation)
    );
    backend.flush(target).unwrap();
    assert_eq!(
        backend.durable_phase(DurableSlot::A),
        InMemoryDurableSlotPhase::Prepared
    );
}

#[test]
fn every_interruption_recovers_a_flush_boundary() {
    let expectations = [
        (1, InMemoryDurableSlotPhase::Empty),
        (2, InMemoryDurableSlotPhase::Empty),
        (3, InMemoryDurableSlotPhase::Prepared),
        (4, InMemoryDurableSlotPhase::Prepared),
        (5, InMemoryDurableSlotPhase::Body),
        (6, InMemoryDurableSlotPhase::Body),
        (7, InMemoryDurableSlotPhase::Body),
        (8, InMemoryDurableSlotPhase::Committed),
    ];

    for (operation, expected) in expectations {
        let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
        backend.inject_interrupt_after(operation).unwrap();

        assert_eq!(
            drive_transaction(&mut backend, target(1)),
            Err(DurableStateBackendError::Interrupted)
        );
        assert_eq!(backend.durable_phase(DurableSlot::A), expected);
        assert_eq!(backend.operation_count(), operation);
        assert_eq!(
            backend.active_generation(),
            (expected == InMemoryDurableSlotPhase::Committed).then_some(1)
        );
    }
}

#[test]
fn inactive_slot_failure_preserves_the_previous_committed_generation() {
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    drive_transaction(&mut backend, target(1)).unwrap();
    assert_eq!(backend.active_generation(), Some(1));

    backend.inject_interrupt_after(7).unwrap();
    assert_eq!(
        drive_transaction(&mut backend, target(2)),
        Err(DurableStateBackendError::Interrupted)
    );
    assert_eq!(backend.active_generation(), Some(1));
    assert_eq!(
        backend.durable_phase(DurableSlot::B),
        InMemoryDurableSlotPhase::Body
    );
    assert_eq!(backend.durable_body_length(DurableSlot::B), 32);

    drive_transaction(&mut backend, target(2)).unwrap();
    assert_eq!(backend.active_generation(), Some(2));
    assert!(backend
        .write(
            DurableSlotWrite::new(
                target(3),
                DurableSlotRegion::PreparedHeader,
                &[0x33; DURABLE_SLOT_HEADER_BYTES],
            )
            .unwrap(),
        )
        .is_ok());
}

#[test]
fn active_committed_slot_cannot_be_overwritten() {
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    drive_transaction(&mut backend, target(1)).unwrap();

    assert_eq!(
        backend.write(
            DurableSlotWrite::new(
                target(3),
                DurableSlotRegion::PreparedHeader,
                &[0x33; DURABLE_SLOT_HEADER_BYTES],
            )
            .unwrap(),
        ),
        Err(DurableStateBackendError::ActiveGenerationConflict)
    );
}

#[test]
fn readback_exposes_only_flushed_bytes() {
    let mut backend = InMemoryDurableStateBackend::new(STORAGE).unwrap();
    let target = target(1);
    let header = [0x61; DURABLE_SLOT_HEADER_BYTES];
    let body = [0x62; 8];
    let mut output = vec![0xff; DURABLE_SLOT_BYTES];

    backend
        .write(DurableSlotWrite::new(target, DurableSlotRegion::PreparedHeader, &header).unwrap())
        .unwrap();
    backend
        .read_slot(STORAGE, DurableSlot::A, &mut output)
        .unwrap();
    assert!(output.iter().all(|byte| *byte == 0));

    backend.flush(target).unwrap();
    backend
        .read_slot(STORAGE, DurableSlot::A, &mut output)
        .unwrap();
    assert_eq!(&output[..DURABLE_SLOT_HEADER_BYTES], &header);

    backend
        .write(DurableSlotWrite::new(target, DurableSlotRegion::Body, &body).unwrap())
        .unwrap();
    backend
        .read_slot(STORAGE, DurableSlot::A, &mut output)
        .unwrap();
    assert!(
        output[DURABLE_SLOT_HEADER_BYTES..DURABLE_SLOT_HEADER_BYTES + body.len()]
            .iter()
            .all(|byte| *byte == 0)
    );
    backend.simulate_power_loss();
    assert_eq!(
        backend.durable_phase(DurableSlot::A),
        InMemoryDurableSlotPhase::Prepared
    );
}
