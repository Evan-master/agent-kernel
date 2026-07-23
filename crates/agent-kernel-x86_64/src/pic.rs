//! Permanent legacy 8259 PIC shutdown for the SMP boot profile.
//!
//! The BSP calls this architecture-binary boundary after I/O APIC routes are
//! installed and while IF remains clear. No SMP runtime path may unmask or
//! reinitialize either controller.

use crate::outb;

const PIC_MASTER_DATA: u16 = 0x21;
const PIC_SLAVE_DATA: u16 = 0xa1;

pub(super) unsafe fn mask_all() {
    unsafe {
        outb(PIC_MASTER_DATA, u8::MAX);
        outb(PIC_SLAVE_DATA, u8::MAX);
    }
}
