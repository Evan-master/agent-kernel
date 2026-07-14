//! Shared legacy 8259 PIC programming for x86 boot interrupt proofs.
//!
//! This architecture-binary module owns the fixed ICW remap sequence and IRQ
//! masks used by source-specific adapters. Callers must hold IF clear; the V0
//! single-core path permits only one exposed legacy IRQ at a time.

use agent_kernel_x86_64::interrupt::{pic_masks_for_irq, PIC_MASTER_OFFSET, PIC_SLAVE_OFFSET};

use crate::outb;

pub(super) const PIC_MASTER_COMMAND: u16 = 0x20;
pub(super) const PIC_MASTER_DATA: u16 = 0x21;
pub(super) const PIC_SLAVE_DATA: u16 = 0xa1;
pub(super) const PIC_EOI: u8 = 0x20;

const PIC_SLAVE_COMMAND: u16 = 0xa0;
const PIC_INITIALIZE: u8 = 0x11;
const PIC_8086_MODE: u8 = 0x01;
const PIC_MASTER_HAS_SLAVE_ON_IRQ2: u8 = 0x04;
const PIC_SLAVE_IDENTITY: u8 = 0x02;
const IO_WAIT_PORT: u16 = 0x80;

pub(super) unsafe fn initialize_for_irq(irq: u8) -> Option<()> {
    let (master_mask, slave_mask) = pic_masks_for_irq(irq)?;
    unsafe {
        outb(PIC_MASTER_COMMAND, PIC_INITIALIZE);
        io_wait();
        outb(PIC_SLAVE_COMMAND, PIC_INITIALIZE);
        io_wait();
        outb(PIC_MASTER_DATA, PIC_MASTER_OFFSET);
        io_wait();
        outb(PIC_SLAVE_DATA, PIC_SLAVE_OFFSET);
        io_wait();
        outb(PIC_MASTER_DATA, PIC_MASTER_HAS_SLAVE_ON_IRQ2);
        io_wait();
        outb(PIC_SLAVE_DATA, PIC_SLAVE_IDENTITY);
        io_wait();
        outb(PIC_MASTER_DATA, PIC_8086_MODE);
        io_wait();
        outb(PIC_SLAVE_DATA, PIC_8086_MODE);
        io_wait();
        outb(PIC_MASTER_DATA, master_mask);
        outb(PIC_SLAVE_DATA, slave_mask);
    }
    Some(())
}

pub(super) unsafe fn mask_all() {
    unsafe {
        outb(PIC_MASTER_DATA, u8::MAX);
        outb(PIC_SLAVE_DATA, u8::MAX);
    }
}

unsafe fn io_wait() {
    unsafe {
        outb(IO_WAIT_PORT, 0);
    }
}
