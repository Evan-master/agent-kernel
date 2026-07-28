//! Exact Q35 PCI preparation for the V28 message-interrupt proof.
//!
//! This owner validates both fixed BDF identities, their conventional
//! capabilities and BAR regions, keeps Bus Master clear during setup, and
//! provides verified MSI/MSI-X activation and quiescence transitions.

mod discovery;

use agent_kernel_x86_64::{
    acpi_topology::DmarPciRequester,
    apic::ApicVector,
    cpu::ApicId,
    pci::{
        program_msix_table_entry, MsiCapability, MsixCapability, MsixTableRegion, PciBar,
        PciCommandGate, PciConfigMechanismOne, PciFunctionAddress, VolatileMsixTable,
        XapicMsiMessage,
    },
    virtio_rng::VirtioRngPciRegions,
    NativePortIo,
};

use crate::agent_memory::PHYSICAL_MEMORY_OFFSET;

use super::{EDU_MSI_VECTOR, RNG_MSIX_VECTOR};

const PCI_BUS: u8 = 0;
const EDU_DEVICE: u8 = 5;
const RNG_DEVICE: u8 = 6;
const FUNCTION_ZERO: u8 = 0;

type NativeConfig = PciConfigMechanismOne<NativePortIo>;
type NativeCommandGate = PciCommandGate<NativeConfig>;

pub(super) fn prepare(
    bootstrap: &mut crate::smp_boot::SmpBootstrap,
    boot_info: &mut bootloader_api::BootInfo,
) -> Result<PreparedMsiMsixHardware, MsiMsixPciError> {
    discovery::prepare(bootstrap, boot_info)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum MsiMsixPciError {
    DmarUnavailable,
    DmarInvalid,
    UnsupportedHostAddressWidth,
    RemappingUnitMissing,
    SplitRemappingUnits,
    PciInventory,
    PciResources,
    Configuration,
    EduMissing,
    EduBarInvalid,
    RngMissing,
    EduCapability,
    RngCapabilityList,
    RngVirtioCapability,
    RngNotifyCapability,
    RngMsixCapability,
    RngRegion,
    MsixTableBarInvalid,
    AddressOverflow,
    MmioMapping,
    Command,
    Message,
    MsiProgramming,
    MsixProgramming,
}

impl MsiMsixPciError {
    pub(super) const fn diagnostic_marker(self) -> &'static str {
        match self {
            Self::DmarUnavailable => "AGENT_KERNEL_MSI_MSIX_DMAR_MISSING_ERROR",
            Self::DmarInvalid => "AGENT_KERNEL_MSI_MSIX_DMAR_INVALID_ERROR",
            Self::UnsupportedHostAddressWidth => "AGENT_KERNEL_MSI_MSIX_DMAR_WIDTH_ERROR",
            Self::RemappingUnitMissing => "AGENT_KERNEL_MSI_MSIX_DMAR_UNIT_ERROR",
            Self::SplitRemappingUnits => "AGENT_KERNEL_MSI_MSIX_DMAR_SPLIT_ERROR",
            Self::PciInventory => "AGENT_KERNEL_MSI_MSIX_PCI_INVENTORY_ERROR",
            Self::PciResources => "AGENT_KERNEL_MSI_MSIX_PCI_RESOURCE_ERROR",
            Self::Configuration => "AGENT_KERNEL_MSI_MSIX_PCI_CONFIG_ERROR",
            Self::EduMissing => "AGENT_KERNEL_MSI_MSIX_EDU_MISSING_ERROR",
            Self::EduBarInvalid => "AGENT_KERNEL_MSI_MSIX_EDU_BAR_ERROR",
            Self::RngMissing => "AGENT_KERNEL_MSI_MSIX_RNG_MISSING_ERROR",
            Self::EduCapability => "AGENT_KERNEL_MSI_MSIX_EDU_CAPABILITY_ERROR",
            Self::RngCapabilityList => "AGENT_KERNEL_MSI_MSIX_RNG_CAPABILITY_LIST_ERROR",
            Self::RngVirtioCapability => "AGENT_KERNEL_MSI_MSIX_RNG_VIRTIO_CAPABILITY_ERROR",
            Self::RngNotifyCapability => "AGENT_KERNEL_MSI_MSIX_RNG_NOTIFY_CAPABILITY_ERROR",
            Self::RngMsixCapability => "AGENT_KERNEL_MSI_MSIX_RNG_MSIX_CAPABILITY_ERROR",
            Self::RngRegion => "AGENT_KERNEL_MSI_MSIX_RNG_REGION_ERROR",
            Self::MsixTableBarInvalid => "AGENT_KERNEL_MSI_MSIX_TABLE_BAR_ERROR",
            Self::AddressOverflow => "AGENT_KERNEL_MSI_MSIX_ADDRESS_ERROR",
            Self::MmioMapping => "AGENT_KERNEL_MSI_MSIX_MMIO_MAPPING_ERROR",
            Self::Command => "AGENT_KERNEL_MSI_MSIX_COMMAND_GATE_ERROR",
            Self::Message => "AGENT_KERNEL_MSI_MSIX_MESSAGE_ERROR",
            Self::MsiProgramming => "AGENT_KERNEL_MSI_CONFIGURATION_ERROR",
            Self::MsixProgramming => "AGENT_KERNEL_MSIX_CONFIGURATION_ERROR",
        }
    }
}

pub(super) struct PreparedMsiMsixHardware {
    edu_requester: DmarPciRequester,
    rng_requester: DmarPciRequester,
    iommu_base: u64,
    edu_bar: PciBar,
    rng_regions: VirtioRngPciRegions,
    notify_multiplier: u32,
    edu_msi: MsiCapability,
    rng_msix: MsixCapability,
    rng_msix_bar: PciBar,
    rng_msix_region: MsixTableRegion,
    edu_gate: NativeCommandGate,
    rng_gate: NativeCommandGate,
}

impl PreparedMsiMsixHardware {
    pub(super) const fn edu_requester(&self) -> DmarPciRequester {
        self.edu_requester
    }

    pub(super) const fn rng_requester(&self) -> DmarPciRequester {
        self.rng_requester
    }

    pub(super) const fn edu_source_id(&self) -> u16 {
        source_id(self.edu_requester)
    }

    pub(super) const fn rng_source_id(&self) -> u16 {
        source_id(self.rng_requester)
    }

    pub(super) const fn iommu_base(&self) -> u64 {
        self.iommu_base
    }

    pub(super) const fn edu_base(&self) -> u64 {
        self.edu_bar.base()
    }

    pub(super) const fn rng_regions(&self) -> VirtioRngPciRegions {
        self.rng_regions
    }

    pub(super) const fn notify_multiplier(&self) -> u32 {
        self.notify_multiplier
    }

    pub(super) fn enable_memory_decode(&mut self) -> Result<(), MsiMsixPciError> {
        self.edu_gate
            .enable_memory_decode()
            .map_err(|_| MsiMsixPciError::Command)?;
        if self.rng_gate.enable_memory_decode().is_err() {
            let _ = self.edu_gate.quiesce();
            return Err(MsiMsixPciError::Command);
        }
        Ok(())
    }

    pub(super) fn configure_edu_msi(&mut self, destination: ApicId) -> Result<(), MsiMsixPciError> {
        let message = message(destination, EDU_MSI_VECTOR)?;
        let mut config = native_config()?;
        self.edu_msi
            .configure(&mut config, edu_address(), message)
            .map_err(|_| MsiMsixPciError::MsiProgramming)?;
        self.edu_gate
            .disable_intx()
            .map_err(|_| MsiMsixPciError::Command)?;
        Ok(())
    }

    pub(super) fn configure_rng_msix(
        &mut self,
        destination: ApicId,
    ) -> Result<(), MsiMsixPciError> {
        let message = message(destination, RNG_MSIX_VECTOR)?;
        let mut config = native_config()?;
        self.rng_msix
            .prepare(&mut config, rng_address())
            .map_err(|_| MsiMsixPciError::MsixProgramming)?;
        let mapped_bytes =
            usize::try_from(self.rng_msix_bar.size()).map_err(|_| MsiMsixPciError::RngRegion)?;
        // SAFETY: preparation mapped the complete table range uncached and
        // retained exclusive ownership of the selected function.
        let mut table = unsafe {
            VolatileMsixTable::bind(
                mapped_pointer(self.rng_msix_bar.base())?,
                mapped_bytes,
                self.rng_msix_region,
            )
        }
        .map_err(|_| MsiMsixPciError::MsixProgramming)?;
        program_msix_table_entry(&mut table, self.rng_msix, 0, message)
            .map_err(|_| MsiMsixPciError::MsixProgramming)?;
        self.rng_msix
            .enable(&mut config, rng_address())
            .map_err(|_| MsiMsixPciError::MsixProgramming)?;
        self.rng_gate
            .disable_intx()
            .map_err(|_| MsiMsixPciError::Command)?;
        Ok(())
    }

    pub(super) fn activate_bus_master(&mut self) -> Result<(), MsiMsixPciError> {
        self.edu_gate
            .enable_memory_and_bus_master()
            .map_err(|_| MsiMsixPciError::Command)?;
        if self.rng_gate.enable_memory_and_bus_master().is_err() {
            let _ = self.edu_gate.quiesce();
            let _ = self.rng_gate.quiesce();
            return Err(MsiMsixPciError::Command);
        }
        Ok(())
    }

    pub(super) fn disable_rng_msix(&mut self) -> Result<(), MsiMsixPciError> {
        let mut config = native_config()?;
        self.rng_msix
            .disable(&mut config, rng_address())
            .map_err(|_| MsiMsixPciError::MsixProgramming)
    }

    pub(super) fn disable_edu_msi(&mut self) -> Result<(), MsiMsixPciError> {
        let mut config = native_config()?;
        self.edu_msi
            .disable(&mut config, edu_address())
            .map_err(|_| MsiMsixPciError::MsiProgramming)
    }

    pub(super) fn quiesce_rng(&mut self) -> Result<(), MsiMsixPciError> {
        self.rng_gate
            .quiesce()
            .map_err(|_| MsiMsixPciError::Command)?;
        Ok(())
    }

    pub(super) fn enable_rng_memory_decode(&mut self) -> Result<(), MsiMsixPciError> {
        self.rng_gate
            .enable_memory_decode()
            .map_err(|_| MsiMsixPciError::Command)?;
        Ok(())
    }

    pub(super) fn enable_rng_bus_master(&mut self) -> Result<(), MsiMsixPciError> {
        self.rng_gate
            .enable_memory_and_bus_master()
            .map_err(|_| MsiMsixPciError::Command)?;
        Ok(())
    }

    pub(super) fn quiesce_all(&mut self) -> Result<(), MsiMsixPciError> {
        let edu = self.edu_gate.quiesce();
        let rng = self.rng_gate.quiesce();
        if edu.is_err() || rng.is_err() {
            Err(MsiMsixPciError::Command)
        } else {
            Ok(())
        }
    }
}

fn native_config() -> Result<NativeConfig, MsiMsixPciError> {
    // SAFETY: the BSP owns Configuration Mechanism 1 while IF is clear.
    let io = unsafe { NativePortIo::new() };
    let mut config = PciConfigMechanismOne::new(io);
    config.probe().map_err(|_| MsiMsixPciError::Configuration)?;
    Ok(config)
}

fn message(destination: ApicId, vector: u8) -> Result<XapicMsiMessage, MsiMsixPciError> {
    XapicMsiMessage::new(
        destination,
        ApicVector::new(vector).expect("fixed device vector"),
    )
    .map_err(|_| MsiMsixPciError::Message)
}

fn mapped_pointer(physical: u64) -> Result<*mut u8, MsiMsixPciError> {
    PHYSICAL_MEMORY_OFFSET
        .checked_add(physical)
        .map(|virtual_address| virtual_address as *mut u8)
        .ok_or(MsiMsixPciError::AddressOverflow)
}

const fn source_id(requester: DmarPciRequester) -> u16 {
    (requester.bus() as u16) << 8 | (requester.device() as u16) << 3 | requester.function() as u16
}

fn edu_address() -> PciFunctionAddress {
    PciFunctionAddress::new(PCI_BUS, EDU_DEVICE, FUNCTION_ZERO).expect("fixed EDU BDF")
}

fn rng_address() -> PciFunctionAddress {
    PciFunctionAddress::new(PCI_BUS, RNG_DEVICE, FUNCTION_ZERO).expect("fixed RNG BDF")
}
