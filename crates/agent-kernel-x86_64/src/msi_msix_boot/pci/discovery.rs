//! Q35 discovery, capability decoding, and MMIO-range validation.
//!
//! This preparation phase runs with IF clear and both target functions
//! quiesced. It publishes no runtime controller until all exact identities,
//! BARs, and capability regions have passed validation.

use agent_kernel_x86_64::{
    acpi_topology::DmarPciRequester,
    edu::{EDU_DEVICE_ID, EDU_MMIO_BYTES, EDU_VENDOR_ID},
    pci::{
        discover_pci_capabilities, MsiCapability, MsixCapability, MsixTableRegion, PciBar,
        PciBarIndex, PciBarKind, PciBarSet, PciCommandGate, PciFunctionSelector,
        PCI_CAPABILITY_ID_MSI, PCI_CAPABILITY_ID_MSIX,
    },
    virtio_rng::{virtio_rng_selector, VirtioRngPciCapabilities, VirtioRngPciRegions},
};
use bootloader_api::BootInfo;

use crate::smp_boot::{SmpBootError, SmpBootstrap};

use super::{
    edu_address, native_config, rng_address, MsiMsixPciError, PreparedMsiMsixHardware, EDU_DEVICE,
    FUNCTION_ZERO, PCI_BUS, RNG_DEVICE,
};

const REGISTER_PAGE_BYTES: u64 = 4096;

pub(super) fn prepare(
    bootstrap: &mut SmpBootstrap,
    boot_info: &mut BootInfo,
) -> Result<PreparedMsiMsixHardware, MsiMsixPciError> {
    let dmar = bootstrap
        .dmar_table()
        .map_err(|_| MsiMsixPciError::DmarInvalid)?
        .ok_or(MsiMsixPciError::DmarUnavailable)?;
    if dmar.host_address_width() != 39 {
        return Err(MsiMsixPciError::UnsupportedHostAddressWidth);
    }
    let edu_requester = requester(EDU_DEVICE);
    let rng_requester = requester(RNG_DEVICE);
    let edu_unit = dmar
        .hardware_unit_for(edu_requester)
        .ok_or(MsiMsixPciError::RemappingUnitMissing)?;
    let rng_unit = dmar
        .hardware_unit_for(rng_requester)
        .ok_or(MsiMsixPciError::RemappingUnitMissing)?;
    if edu_unit.register_base() != rng_unit.register_base() {
        return Err(MsiMsixPciError::SplitRemappingUnits);
    }

    bootstrap
        .prepare_pci_inventory()
        .map_err(|_| MsiMsixPciError::PciInventory)?;
    bootstrap
        .prepare_pci_resources()
        .map_err(|_| MsiMsixPciError::PciResources)?;
    let catalog = bootstrap
        .pci_resources()
        .ok_or(MsiMsixPciError::PciResources)?;
    let edu_resources = catalog
        .claim_candidate_for(edu_selector())
        .ok_or(MsiMsixPciError::EduMissing)?;
    let rng_resources = catalog
        .claim_candidate_for(virtio_rng_selector(rng_address()))
        .ok_or(MsiMsixPciError::RngMissing)?;
    let edu_bar = validate_edu_bar(edu_resources.bars())?;
    let rng_bars = rng_resources.bars();

    let mut config = native_config()?;
    let edu_list = discover_pci_capabilities(&mut config, edu_address())
        .map_err(|_| MsiMsixPciError::EduCapability)?;
    let edu_msi_record = edu_list
        .find(PCI_CAPABILITY_ID_MSI)
        .ok_or(MsiMsixPciError::EduCapability)?;
    let edu_msi = MsiCapability::decode(&mut config, edu_address(), edu_msi_record)
        .map_err(|_| MsiMsixPciError::EduCapability)?;
    let rng_list = discover_pci_capabilities(&mut config, rng_address())
        .map_err(|_| MsiMsixPciError::RngCapabilityList)?;
    let rng_capabilities = VirtioRngPciCapabilities::decode(&mut config, rng_address(), &rng_list)
        .map_err(|_| MsiMsixPciError::RngVirtioCapability)?;
    let rng_regions = rng_capabilities
        .resolve_regions(rng_bars)
        .map_err(|_| MsiMsixPciError::RngRegion)?;
    let notify_multiplier = rng_capabilities
        .notify()
        .notify_offset_multiplier()
        .ok_or(MsiMsixPciError::RngNotifyCapability)?;
    let rng_msix_record = rng_list
        .find(PCI_CAPABILITY_ID_MSIX)
        .ok_or(MsiMsixPciError::RngMsixCapability)?;
    let rng_msix = MsixCapability::decode(&mut config, rng_address(), rng_msix_record)
        .map_err(|_| MsiMsixPciError::RngMsixCapability)?;
    let rng_msix_bar = rng_bars
        .get(rng_msix.table_bar())
        .filter(|bar| is_memory_bar(*bar) && bar.is_assigned())
        .ok_or(MsiMsixPciError::MsixTableBarInvalid)?;
    let rng_msix_region = rng_msix
        .table_region(rng_msix_bar.index(), rng_msix_bar.size())
        .map_err(|_| MsiMsixPciError::MsixTableBarInvalid)?;

    let mut edu_gate = PciCommandGate::bind(native_config()?, edu_address());
    let mut rng_gate = PciCommandGate::bind(native_config()?, rng_address());
    edu_gate.quiesce().map_err(|_| MsiMsixPciError::Command)?;
    rng_gate.quiesce().map_err(|_| MsiMsixPciError::Command)?;

    let ranges = mmio_ranges(
        edu_unit.register_base(),
        edu_bar,
        rng_regions,
        rng_msix_bar,
        rng_msix_region,
    )?;
    bootstrap
        .prepare_dma_mmio_ranges(boot_info, &ranges)
        .map_err(|_: SmpBootError| MsiMsixPciError::MmioMapping)?;

    Ok(PreparedMsiMsixHardware {
        edu_requester,
        rng_requester,
        iommu_base: edu_unit.register_base(),
        edu_bar,
        rng_regions,
        notify_multiplier,
        edu_msi,
        rng_msix,
        rng_msix_bar,
        rng_msix_region,
        edu_gate,
        rng_gate,
    })
}

fn mmio_ranges(
    iommu_base: u64,
    edu_bar: PciBar,
    rng: VirtioRngPciRegions,
    table_bar: PciBar,
    table: MsixTableRegion,
) -> Result<[(u64, u64); 6], MsiMsixPciError> {
    Ok([
        (iommu_base, REGISTER_PAGE_BYTES),
        (edu_bar.base(), REGISTER_PAGE_BYTES),
        region_range(
            rng.common().bar(),
            rng.common().region().offset(),
            rng.common().region().length(),
        )?,
        region_range(
            rng.notify().bar(),
            rng.notify().region().offset(),
            rng.notify().region().length(),
        )?,
        region_range(
            rng.isr().bar(),
            rng.isr().region().offset(),
            rng.isr().region().length(),
        )?,
        region_range(table_bar, table.offset(), table.byte_len())?,
    ])
}

fn region_range(bar: PciBar, offset: u32, length: u32) -> Result<(u64, u64), MsiMsixPciError> {
    let base = bar
        .base()
        .checked_add(u64::from(offset))
        .ok_or(MsiMsixPciError::AddressOverflow)?;
    Ok((base, u64::from(length)))
}

fn validate_edu_bar(bars: PciBarSet) -> Result<PciBar, MsiMsixPciError> {
    let bar = bars
        .get(PciBarIndex::new(0).expect("BAR0"))
        .ok_or(MsiMsixPciError::EduBarInvalid)?;
    if !is_memory_bar(bar) || !bar.is_assigned() || bar.size() != EDU_MMIO_BYTES {
        return Err(MsiMsixPciError::EduBarInvalid);
    }
    Ok(bar)
}

fn is_memory_bar(bar: PciBar) -> bool {
    matches!(
        bar.kind(),
        PciBarKind::MemoryBelowOneMegabyte { .. }
            | PciBarKind::Memory32 { .. }
            | PciBarKind::Memory64 { .. }
    )
}

fn requester(device: u8) -> DmarPciRequester {
    DmarPciRequester::new(0, PCI_BUS, device, FUNCTION_ZERO).expect("fixed Q35 requester")
}

fn edu_selector() -> PciFunctionSelector {
    PciFunctionSelector::new(edu_address(), EDU_VENDOR_ID, EDU_DEVICE_ID)
        .expect("fixed EDU identity")
}
