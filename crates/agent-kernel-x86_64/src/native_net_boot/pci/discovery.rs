//! Q35 virtio-net discovery, capability decoding, and MMIO validation.
//!
//! Preparation runs with IF clear and Bus Master disabled. It publishes no
//! runtime hardware owner until every identity and region is validated.

use agent_kernel_x86_64::{
    acpi_topology::DmarPciRequester,
    pci::{
        discover_pci_capabilities, MsixCapability, PciBar, PciBarKind, PciCommandGate,
        PCI_CAPABILITY_ID_MSIX,
    },
    virtio_net::{virtio_net_selector, VirtioNetPciCapabilities, VirtioNetPciRegions},
};
use bootloader_api::BootInfo;

use crate::smp_boot::{SmpBootError, SmpBootstrap};

use super::{
    native_config, net_address, NativeNetPciError, PreparedNativeNetHardware, FUNCTION_ZERO,
    NET_DEVICE, PCI_BUS,
};

const REGISTER_PAGE_BYTES: u64 = 4096;

pub(super) fn prepare(
    bootstrap: &mut SmpBootstrap,
    boot_info: &mut BootInfo,
) -> Result<PreparedNativeNetHardware, NativeNetPciError> {
    let dmar = bootstrap
        .dmar_table()
        .map_err(|_| NativeNetPciError::DmarInvalid)?
        .ok_or(NativeNetPciError::DmarUnavailable)?;
    if dmar.host_address_width() != 39 {
        return Err(NativeNetPciError::UnsupportedHostAddressWidth);
    }
    let requester =
        DmarPciRequester::new(0, PCI_BUS, NET_DEVICE, FUNCTION_ZERO).expect("fixed Q35 requester");
    let unit = dmar
        .hardware_unit_for(requester)
        .ok_or(NativeNetPciError::RemappingUnitMissing)?;

    bootstrap
        .prepare_pci_inventory()
        .map_err(|_| NativeNetPciError::PciInventory)?;
    bootstrap
        .prepare_pci_resources()
        .map_err(|_| NativeNetPciError::PciResources)?;
    let resources = bootstrap
        .pci_resources()
        .and_then(|catalog| catalog.claim_candidate_for(virtio_net_selector(net_address())))
        .ok_or(NativeNetPciError::NetMissing)?;
    let bars = resources.bars();

    let mut config = native_config()?;
    let list = discover_pci_capabilities(&mut config, net_address())
        .map_err(|_| NativeNetPciError::CapabilityList)?;
    let capabilities = VirtioNetPciCapabilities::decode(&mut config, net_address(), &list)
        .map_err(|_| NativeNetPciError::VirtioCapability)?;
    let regions = capabilities
        .resolve_regions(bars)
        .map_err(|_| NativeNetPciError::Region)?;
    let notify_multiplier = capabilities
        .notify()
        .notify_offset_multiplier()
        .ok_or(NativeNetPciError::NotifyCapability)?;
    let msix_record = list
        .find(PCI_CAPABILITY_ID_MSIX)
        .ok_or(NativeNetPciError::MsixCapability)?;
    let msix = MsixCapability::decode(&mut config, net_address(), msix_record)
        .map_err(|_| NativeNetPciError::MsixCapability)?;
    if msix.table_size() < 2 {
        return Err(NativeNetPciError::MsixTableTooSmall);
    }
    let msix_bar = bars
        .get(msix.table_bar())
        .filter(|bar| is_memory_bar(*bar) && bar.is_assigned())
        .ok_or(NativeNetPciError::MsixTableBarInvalid)?;
    let msix_region = msix
        .table_region(msix_bar.index(), msix_bar.size())
        .map_err(|_| NativeNetPciError::MsixTableBarInvalid)?;

    let mut gate = PciCommandGate::bind(native_config()?, net_address());
    gate.quiesce().map_err(|_| NativeNetPciError::Command)?;
    let ranges = mmio_ranges(unit.register_base(), regions, msix_bar, msix_region)?;
    bootstrap
        .prepare_dma_mmio_ranges(boot_info, &ranges)
        .map_err(|_: SmpBootError| NativeNetPciError::MmioMapping)?;

    Ok(PreparedNativeNetHardware {
        requester,
        iommu_base: unit.register_base(),
        regions,
        notify_multiplier,
        msix,
        msix_bar,
        msix_region,
        gate,
    })
}

fn mmio_ranges(
    iommu_base: u64,
    net: VirtioNetPciRegions,
    table_bar: PciBar,
    table: agent_kernel_x86_64::pci::MsixTableRegion,
) -> Result<[(u64, u64); 6], NativeNetPciError> {
    Ok([
        (iommu_base, REGISTER_PAGE_BYTES),
        region_range(
            net.common().bar(),
            net.common().region().offset(),
            net.common().region().length(),
        )?,
        region_range(
            net.notify().bar(),
            net.notify().region().offset(),
            net.notify().region().length(),
        )?,
        region_range(
            net.isr().bar(),
            net.isr().region().offset(),
            net.isr().region().length(),
        )?,
        region_range(
            net.device().bar(),
            net.device().region().offset(),
            net.device().region().length(),
        )?,
        region_range(table_bar, table.offset(), table.byte_len())?,
    ])
}

fn region_range(bar: PciBar, offset: u32, length: u32) -> Result<(u64, u64), NativeNetPciError> {
    let base = bar
        .base()
        .checked_add(u64::from(offset))
        .ok_or(NativeNetPciError::AddressOverflow)?;
    Ok((base, u64::from(length)))
}

fn is_memory_bar(bar: PciBar) -> bool {
    matches!(
        bar.kind(),
        PciBarKind::MemoryBelowOneMegabyte { .. }
            | PciBarKind::Memory32 { .. }
            | PciBarKind::Memory64 { .. }
    )
}
