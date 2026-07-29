//! Exact Q35 PCI preparation for the V29 native network proof.
//!
//! This owner validates the fixed virtio-net BDF, capability regions, two
//! MSI-X entries, command gating, and all supervisor MMIO mappings.

mod discovery;

use agent_kernel_x86_64::{
    acpi_topology::DmarPciRequester,
    apic::ApicVector,
    cpu::ApicId,
    pci::{
        program_msix_table_entry, MsixCapability, MsixTableRegion, PciBar, PciCommandGate,
        PciConfigMechanismOne, PciFunctionAddress, VolatileMsixTable, XapicMsiMessage,
    },
    virtio_net::VirtioNetPciRegions,
    NativePortIo,
};

use crate::agent_memory::PHYSICAL_MEMORY_OFFSET;

use super::{NET_RX_MSIX_VECTOR, NET_TX_MSIX_VECTOR};

const PCI_BUS: u8 = 0;
const NET_DEVICE: u8 = 5;
const FUNCTION_ZERO: u8 = 0;

type NativeConfig = PciConfigMechanismOne<NativePortIo>;
type NativeCommandGate = PciCommandGate<NativeConfig>;

pub(super) fn prepare(
    bootstrap: &mut crate::smp_boot::SmpBootstrap,
    boot_info: &mut bootloader_api::BootInfo,
) -> Result<PreparedNativeNetHardware, NativeNetPciError> {
    discovery::prepare(bootstrap, boot_info)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum NativeNetPciError {
    DmarUnavailable,
    DmarInvalid,
    UnsupportedHostAddressWidth,
    RemappingUnitMissing,
    PciInventory,
    PciResources,
    Configuration,
    NetMissing,
    CapabilityList,
    VirtioCapability,
    NotifyCapability,
    MsixCapability,
    MsixTableTooSmall,
    Region,
    MsixTableBarInvalid,
    AddressOverflow,
    MmioMapping,
    Command,
    Message,
    MsixProgramming,
}

impl NativeNetPciError {
    pub(super) const fn diagnostic_marker(self) -> &'static str {
        match self {
            Self::DmarUnavailable => "AGENT_KERNEL_NATIVE_NET_DMAR_MISSING_ERROR",
            Self::DmarInvalid => "AGENT_KERNEL_NATIVE_NET_DMAR_INVALID_ERROR",
            Self::UnsupportedHostAddressWidth => "AGENT_KERNEL_NATIVE_NET_DMAR_WIDTH_ERROR",
            Self::RemappingUnitMissing => "AGENT_KERNEL_NATIVE_NET_DMAR_UNIT_ERROR",
            Self::PciInventory => "AGENT_KERNEL_NATIVE_NET_PCI_INVENTORY_ERROR",
            Self::PciResources => "AGENT_KERNEL_NATIVE_NET_PCI_RESOURCE_ERROR",
            Self::Configuration => "AGENT_KERNEL_NATIVE_NET_PCI_CONFIG_ERROR",
            Self::NetMissing => "AGENT_KERNEL_NATIVE_NET_DEVICE_MISSING_ERROR",
            Self::CapabilityList => "AGENT_KERNEL_NATIVE_NET_CAPABILITY_LIST_ERROR",
            Self::VirtioCapability => "AGENT_KERNEL_NATIVE_NET_VIRTIO_CAPABILITY_ERROR",
            Self::NotifyCapability => "AGENT_KERNEL_NATIVE_NET_NOTIFY_CAPABILITY_ERROR",
            Self::MsixCapability => "AGENT_KERNEL_NATIVE_NET_MSIX_CAPABILITY_ERROR",
            Self::MsixTableTooSmall => "AGENT_KERNEL_NATIVE_NET_MSIX_TABLE_SIZE_ERROR",
            Self::Region => "AGENT_KERNEL_NATIVE_NET_REGION_ERROR",
            Self::MsixTableBarInvalid => "AGENT_KERNEL_NATIVE_NET_MSIX_TABLE_BAR_ERROR",
            Self::AddressOverflow => "AGENT_KERNEL_NATIVE_NET_ADDRESS_ERROR",
            Self::MmioMapping => "AGENT_KERNEL_NATIVE_NET_MMIO_MAPPING_ERROR",
            Self::Command => "AGENT_KERNEL_NATIVE_NET_COMMAND_GATE_ERROR",
            Self::Message => "AGENT_KERNEL_NATIVE_NET_MESSAGE_ERROR",
            Self::MsixProgramming => "AGENT_KERNEL_NATIVE_NET_MSIX_PROGRAMMING_ERROR",
        }
    }
}

pub(super) struct PreparedNativeNetHardware {
    requester: DmarPciRequester,
    iommu_base: u64,
    regions: VirtioNetPciRegions,
    notify_multiplier: u32,
    msix: MsixCapability,
    msix_bar: PciBar,
    msix_region: MsixTableRegion,
    gate: NativeCommandGate,
}

impl PreparedNativeNetHardware {
    pub(super) const fn requester(&self) -> DmarPciRequester {
        self.requester
    }

    pub(super) const fn source_id(&self) -> u16 {
        source_id(self.requester)
    }

    pub(super) const fn iommu_base(&self) -> u64 {
        self.iommu_base
    }

    pub(super) const fn regions(&self) -> VirtioNetPciRegions {
        self.regions
    }

    pub(super) const fn notify_multiplier(&self) -> u32 {
        self.notify_multiplier
    }

    pub(super) fn enable_memory_decode(&mut self) -> Result<(), NativeNetPciError> {
        self.gate
            .enable_memory_decode()
            .map_err(|_| NativeNetPciError::Command)?;
        Ok(())
    }

    pub(super) fn configure_msix(&mut self, destination: ApicId) -> Result<(), NativeNetPciError> {
        let rx = message(destination, NET_RX_MSIX_VECTOR)?;
        let tx = message(destination, NET_TX_MSIX_VECTOR)?;
        let mut config = native_config()?;
        self.msix
            .prepare(&mut config, net_address())
            .map_err(|_| NativeNetPciError::MsixProgramming)?;
        let mapped_bytes =
            usize::try_from(self.msix_bar.size()).map_err(|_| NativeNetPciError::Region)?;
        // SAFETY: discovery mapped the complete table range uncached and this
        // owner retains exclusive function ownership.
        let mut table = unsafe {
            VolatileMsixTable::bind(
                mapped_pointer(self.msix_bar.base())?,
                mapped_bytes,
                self.msix_region,
            )
        }
        .map_err(|_| NativeNetPciError::MsixProgramming)?;
        program_msix_table_entry(&mut table, self.msix, 0, rx)
            .map_err(|_| NativeNetPciError::MsixProgramming)?;
        program_msix_table_entry(&mut table, self.msix, 1, tx)
            .map_err(|_| NativeNetPciError::MsixProgramming)?;
        self.msix
            .enable(&mut config, net_address())
            .map_err(|_| NativeNetPciError::MsixProgramming)?;
        self.gate
            .disable_intx()
            .map_err(|_| NativeNetPciError::Command)?;
        Ok(())
    }

    pub(super) fn activate_bus_master(&mut self) -> Result<(), NativeNetPciError> {
        self.gate
            .enable_memory_and_bus_master()
            .map_err(|_| NativeNetPciError::Command)?;
        Ok(())
    }

    pub(super) fn disable_msix(&mut self) -> Result<(), NativeNetPciError> {
        self.msix
            .disable(&mut native_config()?, net_address())
            .map_err(|_| NativeNetPciError::MsixProgramming)
    }

    pub(super) fn quiesce(&mut self) -> Result<(), NativeNetPciError> {
        self.gate
            .quiesce()
            .map_err(|_| NativeNetPciError::Command)?;
        Ok(())
    }
}

fn native_config() -> Result<NativeConfig, NativeNetPciError> {
    // SAFETY: the BSP owns Configuration Mechanism 1 while IF is clear.
    let io = unsafe { NativePortIo::new() };
    let mut config = PciConfigMechanismOne::new(io);
    config
        .probe()
        .map_err(|_| NativeNetPciError::Configuration)?;
    Ok(config)
}

fn message(destination: ApicId, vector: u8) -> Result<XapicMsiMessage, NativeNetPciError> {
    XapicMsiMessage::new(
        destination,
        ApicVector::new(vector).expect("fixed network vector"),
    )
    .map_err(|_| NativeNetPciError::Message)
}

fn mapped_pointer(physical: u64) -> Result<*mut u8, NativeNetPciError> {
    PHYSICAL_MEMORY_OFFSET
        .checked_add(physical)
        .map(|virtual_address| virtual_address as *mut u8)
        .ok_or(NativeNetPciError::AddressOverflow)
}

const fn source_id(requester: DmarPciRequester) -> u16 {
    (requester.bus() as u16) << 8 | (requester.device() as u16) << 3 | requester.function() as u16
}

fn net_address() -> PciFunctionAddress {
    PciFunctionAddress::new(PCI_BUS, NET_DEVICE, FUNCTION_ZERO).expect("fixed network BDF")
}
