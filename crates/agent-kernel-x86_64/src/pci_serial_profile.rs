//! Fixed QEMU PCI serial proof profile for the V25 native Driver path.
//!
//! This bare-metal boot configuration pins identity and bounded transport
//! parameters only. Discovery still supplies the BAR address from hardware.

use agent_kernel_x86_64::{
    interrupt::PCI_INTX_IRQ_LINE,
    pci::{PciBarIndex, PciFunctionAddress, PciFunctionSelector, PciInterruptPin},
};

pub(crate) const VENDOR_ID: u16 = 0x1b36;
pub(crate) const DEVICE_ID: u16 = 0x0002;
pub(crate) const BAR_SPAN: u64 = 8;
pub(crate) const TRANSMIT_BYTE: u8 = b'P';
pub(crate) const TRANSMIT_POLL_BUDGET: u32 = 100_000;
pub(crate) const INTERRUPT_LINE: u8 = PCI_INTX_IRQ_LINE;
pub(crate) const INTERRUPT_PIN: PciInterruptPin = PciInterruptPin::IntA;

pub(crate) const fn selector() -> Option<PciFunctionSelector> {
    let Some(address) = PciFunctionAddress::new(0, 4, 0) else {
        return None;
    };
    PciFunctionSelector::new(address, VENDOR_ID, DEVICE_ID)
}

pub(crate) const fn bar_index() -> Option<PciBarIndex> {
    PciBarIndex::new(0)
}
