use std::{cell::RefCell, collections::BTreeMap};

use agent_kernel_x86_64::{
    apic::{
        ApicMmioError, IcrCommand, IoApicMmio, IoApicPolarity, IoApicRedirectionEntry,
        IoApicRedirectionIndex, IoApicTrigger, LocalApicBase, LocalApicMmio, LocalApicRegister,
        Mmio32, APIC_RESCHEDULE_VECTOR, APIC_SPURIOUS_VECTOR, APIC_TIMER_VECTOR,
        APIC_TLB_SHOOTDOWN_VECTOR,
    },
    cpu::ApicId,
};

#[derive(Default)]
struct RecordingMmio {
    registers: RefCell<BTreeMap<u64, u32>>,
    writes: RefCell<Vec<(u64, u32)>>,
}

#[test]
fn local_apic_timer_calibrates_arms_and_masks_in_canonical_order() {
    let backend = RecordingMmio::default();
    let physical_offset = 0xffff_8000_0000_0000;
    let base = LocalApicBase::new(0xfee0_0000).unwrap();
    let virtual_base = base.virtual_address(physical_offset).unwrap();
    backend.preset(
        virtual_base + LocalApicRegister::TimerCurrentCount.offset() as u64,
        u32::MAX - 42_000,
    );
    let mut apic = LocalApicMmio::new(base, physical_offset, backend).unwrap();

    apic.begin_timer_calibration(APIC_RESCHEDULE_VECTOR);
    assert_eq!(apic.timer_current_count(), u32::MAX - 42_000);
    apic.arm_timer_one_shot(APIC_RESCHEDULE_VECTOR, 42_000)
        .unwrap();
    assert_eq!(apic.arm_timer_one_shot(APIC_RESCHEDULE_VECTOR, 0), None);
    apic.mask_timer(APIC_RESCHEDULE_VECTOR);

    assert_eq!(
        apic.backend().writes(),
        vec![
            (
                virtual_base + LocalApicRegister::TimerDivide.offset() as u64,
                0b0011,
            ),
            (
                virtual_base + LocalApicRegister::LvtTimer.offset() as u64,
                (1 << 16) | APIC_RESCHEDULE_VECTOR.get() as u32,
            ),
            (
                virtual_base + LocalApicRegister::TimerInitialCount.offset() as u64,
                u32::MAX,
            ),
            (
                virtual_base + LocalApicRegister::TimerDivide.offset() as u64,
                0b0011,
            ),
            (
                virtual_base + LocalApicRegister::LvtTimer.offset() as u64,
                APIC_RESCHEDULE_VECTOR.get() as u32,
            ),
            (
                virtual_base + LocalApicRegister::TimerInitialCount.offset() as u64,
                42_000,
            ),
            (
                virtual_base + LocalApicRegister::LvtTimer.offset() as u64,
                (1 << 16) | APIC_RESCHEDULE_VECTOR.get() as u32,
            ),
            (
                virtual_base + LocalApicRegister::TimerInitialCount.offset() as u64,
                0,
            ),
        ]
    );
}

impl RecordingMmio {
    fn preset(&self, address: u64, value: u32) {
        self.registers.borrow_mut().insert(address, value);
    }

    fn writes(&self) -> Vec<(u64, u32)> {
        self.writes.borrow().clone()
    }
}

// SAFETY: the fake maps every address to owned host-side test state and keeps
// values alive for the complete controller lifetime.
unsafe impl Mmio32 for RecordingMmio {
    unsafe fn read32(&self, address: u64) -> u32 {
        self.registers.borrow().get(&address).copied().unwrap_or(0)
    }

    unsafe fn write32(&self, address: u64, value: u32) {
        self.registers.borrow_mut().insert(address, value);
        self.writes.borrow_mut().push((address, value));
    }
}

#[test]
fn local_apic_backend_enables_controller_and_reads_identity() {
    let backend = RecordingMmio::default();
    let physical_offset = 0xffff_8000_0000_0000;
    let base = LocalApicBase::new(0xfee0_0000).unwrap();
    let virtual_base = base.virtual_address(physical_offset).unwrap();
    backend.preset(virtual_base + 0x20, 3 << 24);
    backend.preset(virtual_base + 0x30, 0x14 | (5 << 16));
    let mut apic = LocalApicMmio::new(base, physical_offset, backend).unwrap();

    assert_eq!(apic.id(), ApicId::new(3));
    assert_eq!(apic.version_raw(), 0x14 | (5 << 16));
    apic.enable(APIC_SPURIOUS_VECTOR);
    assert_eq!(
        apic.backend().writes(),
        vec![
            (
                virtual_base + LocalApicRegister::TaskPriority.offset() as u64,
                0
            ),
            (
                virtual_base + LocalApicRegister::LvtLint0.offset() as u64,
                1 << 16,
            ),
            (
                virtual_base + LocalApicRegister::Spurious.offset() as u64,
                0x100 | APIC_SPURIOUS_VECTOR.get() as u32,
            ),
        ]
    );
}

#[test]
fn local_apic_backend_sends_icr_high_before_low_and_detects_busy() {
    let backend = RecordingMmio::default();
    let physical_offset = 0xffff_8000_0000_0000;
    let base = LocalApicBase::new(0xfee0_0000).unwrap();
    let virtual_base = base.virtual_address(physical_offset).unwrap();
    let mut apic = LocalApicMmio::new(base, physical_offset, backend).unwrap();
    let command = IcrCommand::fixed(ApicId::new(4), APIC_TLB_SHOOTDOWN_VECTOR).unwrap();

    apic.backend().preset(
        virtual_base + LocalApicRegister::InterruptCommandLow.offset() as u64,
        1 << 12,
    );
    assert_eq!(apic.try_send(command), Err(ApicMmioError::IcrBusy));
    assert!(apic.backend().writes().is_empty());

    apic.backend().preset(
        virtual_base + LocalApicRegister::InterruptCommandLow.offset() as u64,
        0,
    );
    apic.try_send(command).unwrap();
    assert_eq!(
        apic.backend().writes(),
        vec![
            (
                virtual_base + LocalApicRegister::InterruptCommandHigh.offset() as u64,
                command.high(),
            ),
            (
                virtual_base + LocalApicRegister::InterruptCommandLow.offset() as u64,
                command.low(),
            ),
        ]
    );
    apic.end_of_interrupt();
    assert_eq!(
        apic.backend().writes().last(),
        Some(&(
            virtual_base + LocalApicRegister::EndOfInterrupt.offset() as u64,
            0
        ))
    );
}

#[test]
fn io_apic_backend_masks_before_reprogramming_redirection() {
    let backend = RecordingMmio::default();
    let physical_offset = 0xffff_8000_0000_0000;
    let physical_base = 0xfec0_0000;
    let virtual_base = physical_offset + physical_base;
    backend.preset(virtual_base + 0x10, 0x11 | (23 << 16));
    let mut io_apic = IoApicMmio::new(physical_base, physical_offset, backend).unwrap();
    let version = io_apic.version();
    assert_eq!(version.redirection_count(), 24);
    let index = IoApicRedirectionIndex::new(2, version).unwrap();
    let entry = IoApicRedirectionEntry::fixed(
        APIC_TIMER_VECTOR,
        0,
        IoApicPolarity::ActiveHigh,
        IoApicTrigger::Edge,
        false,
    );
    io_apic.write_redirection(index, entry);

    let selector = virtual_base;
    let window = virtual_base + 0x10;
    assert_eq!(
        io_apic.backend().writes(),
        vec![
            (selector, 1),
            (selector, index.low_register() as u32),
            (window, entry.with_masked(true).low()),
            (selector, index.high_register() as u32),
            (window, entry.high()),
            (selector, index.low_register() as u32),
            (window, entry.low()),
        ]
    );
}
