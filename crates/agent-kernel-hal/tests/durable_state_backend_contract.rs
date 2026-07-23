use agent_kernel_core::{DurableSlot, ResourceId, MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES};
use agent_kernel_hal::{
    DurableFlush, DurableSlotReadback, DurableSlotRegion, DurableSlotTarget,
    DurableSlotTargetError, DurableSlotWrite, DurableSlotWriteError, DurableStateBackend,
    DurableStateBackendError, DURABLE_SLOT_BODY_BYTES, DURABLE_SLOT_BYTES,
    DURABLE_SLOT_FOOTER_BYTES, DURABLE_SLOT_HEADER_BYTES,
};

const _: () = assert!(
    MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES
        + agent_kernel_core::DURABLE_ARCHIVE_MANIFEST_BYTES
        + agent_kernel_core::DURABLE_ARCHIVE_SIGNATURE_BYTES
        <= DURABLE_SLOT_BODY_BYTES
);

#[test]
fn slot_geometry_and_payload_reservation_fit_one_64k_capsule() {
    assert_eq!(DURABLE_SLOT_BYTES, 64 * 1024);
    assert_eq!(DURABLE_SLOT_HEADER_BYTES, 64);
    assert_eq!(DURABLE_SLOT_FOOTER_BYTES, 64);
    assert_eq!(
        DURABLE_SLOT_BODY_BYTES,
        DURABLE_SLOT_BYTES - DURABLE_SLOT_HEADER_BYTES - DURABLE_SLOT_FOOTER_BYTES
    );
    assert_eq!(MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES, 64 * 1024 - 512);
}

#[test]
fn target_and_region_writes_are_structurally_bounded() {
    let target = DurableSlotTarget::new(ResourceId::new(7), DurableSlot::A, 1).unwrap();
    assert_eq!(target.storage(), ResourceId::new(7));
    assert_eq!(target.slot(), DurableSlot::A);
    assert_eq!(target.generation(), 1);
    assert_eq!(
        DurableSlotTarget::new(ResourceId::new(7), DurableSlot::B, 1),
        Err(DurableSlotTargetError::SlotGenerationMismatch {
            expected: DurableSlot::A,
            actual: DurableSlot::B,
        })
    );

    let header = [0; DURABLE_SLOT_HEADER_BYTES];
    let footer = [0; DURABLE_SLOT_FOOTER_BYTES];
    let body = [0x5a; 32];
    assert!(DurableSlotWrite::new(target, DurableSlotRegion::PreparedHeader, &header).is_ok());
    assert!(DurableSlotWrite::new(target, DurableSlotRegion::Body, &body).is_ok());
    assert!(DurableSlotWrite::new(target, DurableSlotRegion::CommitFooter, &footer).is_ok());
    assert_eq!(
        DurableSlotWrite::new(
            target,
            DurableSlotRegion::PreparedHeader,
            &header[..header.len() - 1],
        ),
        Err(DurableSlotWriteError::HeaderLengthMismatch {
            length: DURABLE_SLOT_HEADER_BYTES - 1,
            required: DURABLE_SLOT_HEADER_BYTES,
        })
    );
    assert_eq!(
        DurableSlotWrite::new(target, DurableSlotRegion::Body, &[]),
        Err(DurableSlotWriteError::BodyLengthOutOfRange {
            length: 0,
            limit: DURABLE_SLOT_BODY_BYTES,
        })
    );
}

struct RecordingBackend {
    writes: [Option<DurableSlotRegion>; 3],
    write_count: usize,
    epoch: u64,
}

impl DurableStateBackend for RecordingBackend {
    fn write(&mut self, request: DurableSlotWrite<'_>) -> Result<(), DurableStateBackendError> {
        self.writes[self.write_count] = Some(request.region());
        self.write_count += 1;
        Ok(())
    }

    fn flush(
        &mut self,
        target: DurableSlotTarget,
    ) -> Result<DurableFlush, DurableStateBackendError> {
        self.epoch += 1;
        Ok(DurableFlush::new(target, self.epoch).unwrap())
    }

    fn read_slot(
        &mut self,
        storage: ResourceId,
        slot: DurableSlot,
        output: &mut [u8],
    ) -> Result<DurableSlotReadback, DurableStateBackendError> {
        if output.len() < DURABLE_SLOT_BYTES {
            return Err(DurableStateBackendError::BufferTooSmall {
                required: DURABLE_SLOT_BYTES,
                available: output.len(),
            });
        }
        output[..DURABLE_SLOT_BYTES].fill(0xa5);
        Ok(DurableSlotReadback::new(
            storage,
            slot,
            DURABLE_SLOT_BYTES,
            self.epoch,
        ))
    }
}

#[test]
fn backend_contract_preserves_write_flush_readback_phases() {
    let target = DurableSlotTarget::new(ResourceId::new(7), DurableSlot::A, 1).unwrap();
    let mut backend = RecordingBackend {
        writes: [None; 3],
        write_count: 0,
        epoch: 0,
    };
    let header = [0; DURABLE_SLOT_HEADER_BYTES];
    let body = [0x33; 8];
    let footer = [0; DURABLE_SLOT_FOOTER_BYTES];

    backend
        .write(DurableSlotWrite::new(target, DurableSlotRegion::PreparedHeader, &header).unwrap())
        .unwrap();
    assert_eq!(backend.flush(target).unwrap().epoch(), 1);
    backend
        .write(DurableSlotWrite::new(target, DurableSlotRegion::Body, &body).unwrap())
        .unwrap();
    assert_eq!(backend.flush(target).unwrap().epoch(), 2);
    backend
        .write(DurableSlotWrite::new(target, DurableSlotRegion::CommitFooter, &footer).unwrap())
        .unwrap();
    assert_eq!(backend.flush(target).unwrap().epoch(), 3);

    let mut output = [0; DURABLE_SLOT_BYTES];
    let readback = backend
        .read_slot(target.storage(), target.slot(), &mut output)
        .unwrap();
    assert_eq!(
        backend.writes,
        [
            Some(DurableSlotRegion::PreparedHeader),
            Some(DurableSlotRegion::Body),
            Some(DurableSlotRegion::CommitFooter),
        ]
    );
    assert_eq!(readback.storage(), target.storage());
    assert_eq!(readback.slot(), target.slot());
    assert_eq!(readback.bytes_read(), DURABLE_SLOT_BYTES);
    assert_eq!(readback.flush_epoch(), 3);
    assert!(output.iter().all(|byte| *byte == 0xa5));
}
