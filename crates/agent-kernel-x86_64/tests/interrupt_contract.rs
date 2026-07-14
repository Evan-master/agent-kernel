use core::mem::size_of;

use agent_kernel_x86_64::interrupt::{
    legacy_irq_vector, pic_masks_for_irq, IdtEntry, IdtPointer, IDT_INTERRUPT_GATE_OPTIONS,
    IDT_TRAP_GATE_OPTIONS, PIC_MASTER_OFFSET, PIC_SLAVE_OFFSET, PIT_CHANNEL0_COMMAND,
    PIT_CHANNEL0_DATA_PORT, PIT_COMMAND_PORT, PIT_DIVISOR, PIT_INPUT_HZ, PIT_IRQ_LINE,
    PIT_IRQ_VECTOR, PIT_TARGET_HZ, UART_IRQ_LINE, UART_IRQ_VECTOR,
};

#[test]
fn long_mode_idt_entry_splits_handler_and_preserves_gate_contract() {
    let handler = 0x1234_5678_9abc_def0;
    let entry = IdtEntry::interrupt_gate(handler, 0x08);

    assert_eq!(size_of::<IdtEntry>(), 16);
    assert_eq!(entry.handler_address(), handler);
    assert_eq!(entry.selector(), 0x08);
    assert_eq!(entry.options(), IDT_INTERRUPT_GATE_OPTIONS);
    assert_eq!(entry.ist(), 0);
    assert_eq!(entry.reserved(), 0);
}

#[test]
fn long_mode_trap_gate_preserves_returning_exception_contract() {
    let handler = 0xffff_8000_1234_5678;
    let entry = IdtEntry::trap_gate(handler, 0x08);

    assert_eq!(entry.handler_address(), handler);
    assert_eq!(entry.selector(), 0x08);
    assert_eq!(entry.options(), IDT_TRAP_GATE_OPTIONS);
    assert_eq!(entry.ist(), 0);
    assert_eq!(entry.reserved(), 0);
}

#[test]
fn idtr_descriptor_covers_exact_fixed_table_bytes() {
    let pointer = IdtPointer::for_table(0x1234_5000, 256).unwrap();

    assert_eq!(size_of::<IdtPointer>(), 10);
    assert_eq!(pointer.base(), 0x1234_5000);
    assert_eq!(pointer.limit(), 4095);
    assert!(IdtPointer::for_table(0, 0).is_none());
    assert!(IdtPointer::for_table(0, 4097).is_none());
}

#[test]
fn legacy_pic_vectors_and_masks_expose_only_requested_irq() {
    assert_eq!(PIC_MASTER_OFFSET, 0x20);
    assert_eq!(PIC_SLAVE_OFFSET, 0x28);
    assert_eq!(UART_IRQ_LINE, 4);
    assert_eq!(UART_IRQ_VECTOR, 0x24);
    assert_eq!(legacy_irq_vector(4), Some(UART_IRQ_VECTOR));
    assert_eq!(legacy_irq_vector(12), Some(0x2c));
    assert_eq!(legacy_irq_vector(16), None);

    assert_eq!(pic_masks_for_irq(4), Some((0xef, 0xff)));
    assert_eq!(pic_masks_for_irq(12), Some((0xfb, 0xef)));
    assert_eq!(pic_masks_for_irq(16), None);
}

#[test]
fn pit_channel_zero_contract_targets_one_hundred_hertz_irq_zero() {
    assert_eq!(PIT_CHANNEL0_DATA_PORT, 0x40);
    assert_eq!(PIT_COMMAND_PORT, 0x43);
    assert_eq!(PIT_CHANNEL0_COMMAND, 0x36);
    assert_eq!(PIT_INPUT_HZ, 1_193_182);
    assert_eq!(PIT_TARGET_HZ, 100);
    assert_eq!(PIT_DIVISOR, 11_932);
    assert_eq!(PIT_DIVISOR.to_le_bytes(), [0x9c, 0x2e]);
    assert_eq!(PIT_IRQ_LINE, 0);
    assert_eq!(PIT_IRQ_VECTOR, 0x20);
    assert_eq!(legacy_irq_vector(PIT_IRQ_LINE), Some(PIT_IRQ_VECTOR));
    assert_eq!(pic_masks_for_irq(PIT_IRQ_LINE), Some((0xfe, 0xff)));
}
