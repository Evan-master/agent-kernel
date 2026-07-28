//! BSP-owned DMAR handoff and DMA MMIO mapping.
//!
//! This boot child exposes the strictly parsed DMAR snapshot and maps only the
//! selected VT-d register page and PCI DMA register page before AP startup.

use agent_kernel_x86_64::acpi_topology::AcpiDmarDiscoveryError;
use bootloader_api::BootInfo;

use super::{memory, BootDmarTable, SmpBootError, SmpBootstrap};

impl SmpBootstrap {
    pub(crate) const fn dmar_table(&self) -> Result<Option<BootDmarTable>, AcpiDmarDiscoveryError> {
        self.dmar_table
    }

    pub(crate) fn prepare_dma_mmio(
        &mut self,
        boot_info: &mut BootInfo,
        iommu_base: u64,
        device_base: u64,
    ) -> Result<(), SmpBootError> {
        memory::map_dma_mmio_pages(boot_info, iommu_base, device_base)
            .map_err(SmpBootError::ApicMapping)
    }
}
