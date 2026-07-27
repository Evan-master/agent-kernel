//! BSP-owned native PCI configuration discovery.
//!
//! This boot adapter creates the only Configuration Mechanism 1 owner while
//! interrupts are disabled, validates the address latch, and returns a bounded
//! immutable inventory for the remainder of the boot.

use agent_kernel_x86_64::{
    pci::{
        discover_pci_functions, PciConfigMechanismOne, PciConfigMechanismOneError,
        PciDiscoveryError, PciInventory,
    },
    NativePortIo,
};

use super::{SmpBootError, SmpBootstrap};

pub(super) const PCI_FUNCTION_CAPACITY: usize = 256;
pub(super) type BootPciInventory = PciInventory<PCI_FUNCTION_CAPACITY>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PciBootError {
    Configuration(PciConfigMechanismOneError),
    Discovery(PciDiscoveryError),
}

pub(super) fn discover() -> Result<BootPciInventory, PciBootError> {
    // SAFETY: this runs on the BSP with IF clear before AP startup. The value is
    // bound immediately to the fixed PCI configuration adapter.
    let io = unsafe { NativePortIo::new() };
    let mut config = PciConfigMechanismOne::new(io);
    config.probe().map_err(PciBootError::Configuration)?;
    discover_pci_functions(&mut config).map_err(PciBootError::Discovery)
}

impl SmpBootstrap {
    pub(crate) fn prepare_pci_inventory(&mut self) -> Result<usize, SmpBootError> {
        if self.pci_inventory.is_some() {
            return Err(SmpBootError::PciAlreadyDiscovered);
        }
        let inventory = discover().map_err(SmpBootError::Pci)?;
        let function_count = inventory.len();
        self.pci_inventory = Some(inventory);
        Ok(function_count)
    }

    pub(crate) const fn pci_inventory(&self) -> Option<&BootPciInventory> {
        self.pci_inventory.as_ref()
    }
}
