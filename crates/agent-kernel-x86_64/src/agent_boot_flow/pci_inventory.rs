//! Native PCI inventory preparation and boot evidence.

use crate::{fatal_boot, serial_write_line, smp_boot::SmpBootstrap};

pub(super) fn prepare(smp_bootstrap: &mut SmpBootstrap) {
    let function_count = smp_bootstrap
        .prepare_pci_inventory()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_PCI_DISCOVERY_ERROR"));
    if smp_bootstrap
        .pci_inventory()
        .map(|inventory| inventory.len())
        != Some(function_count)
    {
        fatal_boot("AGENT_KERNEL_PCI_INVENTORY_ERROR");
    }
    let resource_count = smp_bootstrap
        .prepare_pci_resources()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_PCI_BAR_PROBE_ERROR"));
    if smp_bootstrap
        .pci_resources()
        .map(|resources| resources.len())
        != Some(resource_count)
    {
        fatal_boot("AGENT_KERNEL_PCI_RESOURCE_CATALOG_ERROR");
    }
    serial_write_line("AGENT_KERNEL_PCI_CONFIG_IO_OK");
    serial_write_line("AGENT_KERNEL_PCI_INVENTORY_OK");
    serial_write_line("AGENT_KERNEL_PCI_BAR_CATALOG_OK");
}
