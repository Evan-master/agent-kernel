//! Q35 DMAR and EDU PCI preparation for the native proof.

use agent_kernel_x86_64::{
    acpi_topology::DmarPciRequester,
    edu::{EDU_DEVICE_ID, EDU_MMIO_BYTES, EDU_VENDOR_ID},
    pci::{
        PciBarIndex, PciBarKind, PciCommandGate, PciCommandGateError, PciCommandState,
        PciConfigMechanismOne, PciConfigMechanismOneError, PciFunctionAddress, PciFunctionSelector,
    },
    NativePortIo,
};
use bootloader_api::BootInfo;

use crate::smp_boot::{SmpBootError, SmpBootstrap};

const EDU_BUS: u8 = 0;
const EDU_DEVICE: u8 = 5;
const EDU_FUNCTION: u8 = 0;

type NativeCommandGate = PciCommandGate<PciConfigMechanismOne<NativePortIo>>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum DmaPciError {
    DmarUnavailable,
    DmarInvalid,
    UnsupportedHostAddressWidth(u8),
    RemappingUnitMissing,
    Pci(SmpBootError),
    Configuration(PciConfigMechanismOneError),
    EduMissing,
    EduBarMissing,
    EduBarInvalid,
    Command(PciCommandGateError),
}

pub(super) struct PreparedDmaHardware {
    requester: DmarPciRequester,
    source_id: u16,
    iommu_base: u64,
    edu_base: u64,
    gate: NativeCommandGate,
}

impl PreparedDmaHardware {
    pub(super) const fn requester(&self) -> DmarPciRequester {
        self.requester
    }

    pub(super) const fn source_id(&self) -> u16 {
        self.source_id
    }

    pub(super) const fn iommu_base(&self) -> u64 {
        self.iommu_base
    }

    pub(super) const fn edu_base(&self) -> u64 {
        self.edu_base
    }

    pub(super) fn enable(&mut self) -> Result<PciCommandState, DmaPciError> {
        self.gate
            .enable_memory_and_bus_master()
            .map_err(DmaPciError::Command)
    }

    pub(super) fn quiesce(&mut self) -> Result<PciCommandState, DmaPciError> {
        self.gate.quiesce().map_err(DmaPciError::Command)
    }
}

pub(super) fn prepare(
    bootstrap: &mut SmpBootstrap,
    boot_info: &mut BootInfo,
) -> Result<PreparedDmaHardware, DmaPciError> {
    let dmar = bootstrap
        .dmar_table()
        .map_err(|_| DmaPciError::DmarInvalid)?
        .ok_or(DmaPciError::DmarUnavailable)?;
    if dmar.host_address_width() != 39 {
        return Err(DmaPciError::UnsupportedHostAddressWidth(
            dmar.host_address_width(),
        ));
    }
    let requester =
        DmarPciRequester::new(0, EDU_BUS, EDU_DEVICE, EDU_FUNCTION).expect("fixed EDU BDF");
    let unit = dmar
        .hardware_unit_for(requester)
        .ok_or(DmaPciError::RemappingUnitMissing)?;

    bootstrap
        .prepare_pci_inventory()
        .map_err(DmaPciError::Pci)?;
    bootstrap
        .prepare_pci_resources()
        .map_err(DmaPciError::Pci)?;
    let address =
        PciFunctionAddress::new(EDU_BUS, EDU_DEVICE, EDU_FUNCTION).expect("fixed EDU BDF");
    let selector = PciFunctionSelector::new(address, EDU_VENDOR_ID, EDU_DEVICE_ID)
        .expect("fixed EDU identity");
    let resources = bootstrap
        .pci_resources()
        .and_then(|catalog| catalog.claim_candidate_for(selector))
        .ok_or(DmaPciError::EduMissing)?;
    let bar = resources
        .bars()
        .get(PciBarIndex::new(0).expect("BAR0"))
        .ok_or(DmaPciError::EduBarMissing)?;
    if !matches!(
        bar.kind(),
        PciBarKind::Memory32 { .. } | PciBarKind::Memory64 { .. }
    ) || !bar.is_assigned()
        || bar.size() != EDU_MMIO_BYTES
    {
        return Err(DmaPciError::EduBarInvalid);
    }

    // SAFETY: the BSP owns PCI Configuration Mechanism 1 with interrupts off.
    let io = unsafe { NativePortIo::new() };
    let mut config = PciConfigMechanismOne::new(io);
    config.probe().map_err(DmaPciError::Configuration)?;
    let mut gate = PciCommandGate::bind(config, address);
    gate.quiesce().map_err(DmaPciError::Command)?;
    bootstrap
        .prepare_dma_mmio(boot_info, unit.register_base(), bar.base())
        .map_err(DmaPciError::Pci)?;

    let source_id =
        (u16::from(EDU_BUS) << 8) | (u16::from(EDU_DEVICE) << 3) | u16::from(EDU_FUNCTION);
    Ok(PreparedDmaHardware {
        requester,
        source_id,
        iommu_base: unit.register_base(),
        edu_base: bar.base(),
        gate,
    })
}
