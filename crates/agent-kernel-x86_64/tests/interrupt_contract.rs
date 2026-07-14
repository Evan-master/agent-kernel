use core::mem::size_of;

use agent_kernel_x86_64::interrupt::{
    legacy_irq_vector, pic_masks_for_irq, IdtEntry, IdtPointer, IDT_INTERRUPT_GATE_OPTIONS,
    PIC_MASTER_OFFSET, PIC_SLAVE_OFFSET, UART_IRQ_LINE, UART_IRQ_VECTOR,
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
