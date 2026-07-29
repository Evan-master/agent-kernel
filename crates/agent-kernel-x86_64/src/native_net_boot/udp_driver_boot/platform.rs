//! VT-d, Virtio queue, and MSI-X setup for the V30 driver session.

use agent_kernel_core::{DmaAccess, NetworkMacAddress};
use agent_kernel_x86_64::{
    iommu::{IntelVtd, VolatileVtdMmio, VtdDomainId, VtdLegacyTablePages},
    virtio_net::{VirtioNetDevice, VirtioNetQueueLayout, VolatileVirtioNetDeviceConfig},
    virtio_rng::{VolatileVirtioCommonConfig, VolatileVirtioIsr, VolatileVirtioNotify},
};
use bootloader_api::BootInfo;

use crate::{
    agent_cpu::AgentCpuRuntime, agent_memory::PreparedAgentMemory,
    boot_agent_images::BootNetworkDriverImage, fatal_boot, smp_boot::SmpBootstrap, X86BootedKernel,
};

use super::{super::authority::NativeNetAuthority, admission::NetworkDriverAdmission, session};
use crate::native_net_boot::{
    memory, pci,
    proof::{fatal_after_enable, mapped_bytes, mapped_pointer, publish_dma_memory},
    NET_RX_METADATA_IOVA, NET_RX_PACKET_IOVA, NET_TX_METADATA_IOVA, NET_TX_PACKET_IOVA,
};

const MMIO_POLL_BUDGET: u32 = 100_000_000;

pub(super) type NativeUdpNet<'a> = VirtioNetDevice<
    'a,
    VolatileVirtioCommonConfig,
    VolatileVirtioNotify,
    VolatileVirtioIsr,
    VolatileVirtioNetDeviceConfig,
>;

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    boot_info: &'static mut BootInfo,
    mut smp: SmpBootstrap,
    mut hardware: pci::PreparedNativeNetHardware,
    mut booted: X86BootedKernel,
    authority: NativeNetAuthority,
    driver: NetworkDriverAdmission,
    contract: BootNetworkDriverImage,
    cpu_runtime: AgentCpuRuntime,
    neighbor_memory: PreparedAgentMemory,
    udp_memory: PreparedAgentMemory,
    guest_mac: NetworkMacAddress,
) -> ! {
    let mut pages = memory::allocate(boot_info)
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_FRAME_ALLOCATION_ERROR"));
    let rx_metadata_pointer = pages.rx_metadata_pointer();
    let rx_packet_pointer = pages.rx_packet_pointer();
    let tx_metadata_pointer = pages.tx_metadata_pointer();
    let tx_packet_pointer = pages.tx_packet_pointer();
    let mappings = [
        (
            NET_RX_METADATA_IOVA,
            pages.rx_metadata_physical(),
            DmaAccess::ReadWrite,
        ),
        (
            NET_RX_PACKET_IOVA,
            pages.rx_packet_physical(),
            DmaAccess::Write,
        ),
        (
            NET_TX_METADATA_IOVA,
            pages.tx_metadata_physical(),
            DmaAccess::ReadWrite,
        ),
        (
            NET_TX_PACKET_IOVA,
            pages.tx_packet_physical(),
            DmaAccess::Read,
        ),
    ];
    let mut tables = pages
        .table_pages()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_TABLE_ERROR"));
    configure_tables(&mut tables, &hardware, mappings);
    publish_dma_memory();

    let iommu_pointer = mapped_pointer(hardware.iommu_base())
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_IOMMU_POINTER_ERROR"));
    // SAFETY: PCI discovery mapped the complete DRHD register page uncached.
    let iommu_io = unsafe { VolatileVtdMmio::new(iommu_pointer) }
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_IOMMU_POINTER_ERROR"));
    let mut iommu = IntelVtd::bind(iommu_io, MMIO_POLL_BUDGET)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_IOMMU_BIND_ERROR"));
    iommu
        .activate(tables.root_address())
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_IOMMU_ACTIVATION_ERROR"));
    authority
        .activate(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_AUTHORITY_ACTIVATION_ERROR"));

    hardware
        .enable_memory_decode()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MEMORY_DECODE_ERROR"));
    let regions = hardware.regions();
    // SAFETY: PCI discovery validated every BAR-relative region. The profile
    // retains exclusive ownership of the function and four DMA frames.
    let common = unsafe {
        VolatileVirtioCommonConfig::bind(
            mapped_pointer(regions.common().bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MMIO_POINTER_ERROR")),
            mapped_bytes(regions.common().bar().size()),
            regions.common().region(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MMIO_POINTER_ERROR"));
    let notify = unsafe {
        VolatileVirtioNotify::bind(
            mapped_pointer(regions.notify().bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MMIO_POINTER_ERROR")),
            mapped_bytes(regions.notify().bar().size()),
            regions.notify().region(),
            hardware.notify_multiplier(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MMIO_POINTER_ERROR"));
    let isr = unsafe {
        VolatileVirtioIsr::bind(
            mapped_pointer(regions.isr().bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MMIO_POINTER_ERROR")),
            mapped_bytes(regions.isr().bar().size()),
            regions.isr().region(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MMIO_POINTER_ERROR"));
    let device_config = unsafe {
        VolatileVirtioNetDeviceConfig::bind(
            mapped_pointer(regions.device().bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MMIO_POINTER_ERROR")),
            mapped_bytes(regions.device().bar().size()),
            regions.device().region(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MMIO_POINTER_ERROR"));
    let rx_layout = VirtioNetQueueLayout::new(NET_RX_METADATA_IOVA, NET_RX_PACKET_IOVA)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_QUEUE_LAYOUT_ERROR"));
    let tx_layout = VirtioNetQueueLayout::new(NET_TX_METADATA_IOVA, NET_TX_PACKET_IOVA)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_QUEUE_LAYOUT_ERROR"));
    let mut net = VirtioNetDevice::bind(
        common,
        notify,
        isr,
        device_config,
        MMIO_POLL_BUDGET,
        // SAFETY: allocation returned four disjoint boot-lifetime DMA frames.
        unsafe { &mut *rx_metadata_pointer },
        unsafe { &mut *rx_packet_pointer },
        rx_layout,
        unsafe { &mut *tx_metadata_pointer },
        unsafe { &mut *tx_packet_pointer },
        tx_layout,
    )
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_BIND_ERROR"));
    hardware
        .configure_msix(smp.bsp_apic_id())
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_MSIX_CONFIG_ERROR"));
    if net
        .initialize(0, 1)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_INITIALIZATION_ERROR"))
        != guest_mac
    {
        fatal_boot("AGENT_KERNEL_NATIVE_UDP_MAC_MISMATCH_ERROR");
    }
    hardware.activate_bus_master().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_UDP_BUS_MASTER_ERROR")
    });

    session::run(
        &mut booted,
        authority,
        driver,
        contract,
        cpu_runtime,
        neighbor_memory,
        udp_memory,
        guest_mac,
        &mut smp,
        &mut hardware,
        &mut net,
        &mut iommu,
        &mut tables,
    )
}

fn configure_tables(
    tables: &mut VtdLegacyTablePages<'_>,
    hardware: &pci::PreparedNativeNetHardware,
    mappings: [(u64, u64, DmaAccess); 4],
) {
    let domain = VtdDomainId::new(1).expect("fixed nonzero VT-d domain");
    tables
        .attach_requester(hardware.requester(), domain)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_TABLE_ERROR"));
    for (iova, physical, access) in mappings {
        tables
            .install_mapping(iova, physical, access)
            .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_TABLE_ERROR"));
    }
}
