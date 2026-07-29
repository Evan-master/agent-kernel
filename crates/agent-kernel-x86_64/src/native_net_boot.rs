//! Native QEMU Virtio network, MSI-X, and VT-d closed-loop proof.
//!
//! This V29 profile sends one ARP request through capability-governed queues,
//! validates the gateway reply, then proves detached DMA is denied by VT-d.

mod authority;
mod interrupts;
mod memory;
mod network_proof;
mod pci;
mod proof;

use agent_kernel_core::{DmaAccess, NetworkMacAddress};
use agent_kernel_x86_64::{
    iommu::{IntelVtd, VolatileVtdMmio, VtdDomainId},
    virtio_net::{
        build_arp_request, is_expected_arp_reply, VirtioNetDevice, VirtioNetQueueLayout,
        VolatileVirtioNetDeviceConfig, ARP_FRAME_BYTES,
    },
    virtio_rng::{VolatileVirtioCommonConfig, VolatileVirtioIsr, VolatileVirtioNotify},
};
use bootloader_api::BootInfo;

use crate::{
    exception_runtime, exit_qemu, fatal_boot, halt_forever, privilege_runtime::PrivilegeBoundary,
    serial_write_line, smp_boot::SmpBootstrap,
};

use self::network_proof::{frame_descriptor, run_detached_dma_probe, rx_error_marker};
use self::proof::{
    fatal_after_enable, mapped_bytes, mapped_pointer, publish_dma_memory, require_no_fault,
};

pub(super) const NET_RX_METADATA_IOVA: u64 = 0x0200_0000;
pub(super) const NET_RX_PACKET_IOVA: u64 = 0x0200_1000;
pub(super) const NET_TX_METADATA_IOVA: u64 = 0x0200_2000;
pub(super) const NET_TX_PACKET_IOVA: u64 = 0x0200_3000;
pub(super) const NET_RX_MSIX_VECTOR: u8 = 0xd2;
pub(super) const NET_TX_MSIX_VECTOR: u8 = 0xd3;

const GUEST_MAC_BYTES: [u8; 6] = [0x52, 0x54, 0, 0x12, 0x34, 0x56];
const MMIO_POLL_BUDGET: u32 = 100_000_000;
pub(super) const FAULT_WAIT_SPINS: usize = 100_000_000;

pub(super) fn run(
    boot_info: &'static mut BootInfo,
    privilege_boundary: PrivilegeBoundary,
    mut smp_bootstrap: SmpBootstrap,
) -> ! {
    let _privilege_boundary = privilege_boundary;
    interrupts::install_gates().unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_IDT_ERROR"));
    smp_bootstrap
        .prepare_apic_mmio(boot_info)
        .unwrap_or_else(|error| fatal_boot(error.diagnostic_marker()));
    exception_runtime::freeze_for_smp()
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_IDT_FREEZE_ERROR"));

    let mut hardware = pci::prepare(&mut smp_bootstrap, boot_info)
        .unwrap_or_else(|error| fatal_boot(error.diagnostic_marker()));
    serial_write_line("AGENT_KERNEL_NATIVE_NET_DMAR_DISCOVERY_OK");
    let expected_mac =
        NetworkMacAddress::new(GUEST_MAC_BYTES).expect("fixed unicast network identity");
    let (mut booted, authority) = authority::reserve(
        smp_bootstrap.bsp_apic_id().get(),
        hardware.source_id(),
        expected_mac,
    )
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_AUTHORITY_ERROR"));
    serial_write_line("AGENT_KERNEL_NATIVE_NET_CAPABILITY_OK");

    let mut pages = memory::allocate(boot_info)
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_FRAME_ALLOCATION_ERROR"));
    let rx_metadata_pointer = pages.rx_metadata_pointer();
    let rx_packet_pointer = pages.rx_packet_pointer();
    let tx_metadata_pointer = pages.tx_metadata_pointer();
    let tx_packet_pointer = pages.tx_packet_pointer();
    let rx_metadata_physical = pages.rx_metadata_physical();
    let rx_packet_physical = pages.rx_packet_physical();
    let tx_metadata_physical = pages.tx_metadata_physical();
    let tx_packet_physical = pages.tx_packet_physical();
    let mut tables = pages
        .table_pages()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_TABLE_ERROR"));
    let domain = VtdDomainId::new(1).expect("fixed nonzero VT-d domain");
    tables
        .attach_requester(hardware.requester(), domain)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_TABLE_ERROR"));
    for (iova, physical, access) in [
        (
            NET_RX_METADATA_IOVA,
            rx_metadata_physical,
            DmaAccess::ReadWrite,
        ),
        (NET_RX_PACKET_IOVA, rx_packet_physical, DmaAccess::Write),
        (
            NET_TX_METADATA_IOVA,
            tx_metadata_physical,
            DmaAccess::ReadWrite,
        ),
        (NET_TX_PACKET_IOVA, tx_packet_physical, DmaAccess::Read),
    ] {
        tables
            .install_mapping(iova, physical, access)
            .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_TABLE_ERROR"));
    }
    publish_dma_memory();

    let iommu_pointer = mapped_pointer(hardware.iommu_base())
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_IOMMU_POINTER_ERROR"));
    // SAFETY: PCI discovery mapped the complete DRHD register page uncached.
    let iommu_io = unsafe { VolatileVtdMmio::new(iommu_pointer) }
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_IOMMU_POINTER_ERROR"));
    let mut iommu = IntelVtd::bind(iommu_io, MMIO_POLL_BUDGET)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_IOMMU_BIND_ERROR"));
    iommu
        .activate(tables.root_address())
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_IOMMU_ACTIVATION_ERROR"));
    authority
        .activate(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_AUTHORITY_ACTIVATION_ERROR"));
    serial_write_line("AGENT_KERNEL_NATIVE_NET_DMA_DOMAIN_OK");

    hardware
        .enable_memory_decode()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MEMORY_DECODE_ERROR"));
    let regions = hardware.regions();
    let common_region = regions.common();
    let notify_region = regions.notify();
    let isr_region = regions.isr();
    let device_region = regions.device();
    // SAFETY: PCI discovery validated and mapped every BAR-relative region,
    // and this proof retains exclusive ownership of the function.
    let common = unsafe {
        VolatileVirtioCommonConfig::bind(
            mapped_pointer(common_region.bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR")),
            mapped_bytes(common_region.bar().size()),
            common_region.region(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR"));
    let notify = unsafe {
        VolatileVirtioNotify::bind(
            mapped_pointer(notify_region.bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR")),
            mapped_bytes(notify_region.bar().size()),
            notify_region.region(),
            hardware.notify_multiplier(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR"));
    let isr = unsafe {
        VolatileVirtioIsr::bind(
            mapped_pointer(isr_region.bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR")),
            mapped_bytes(isr_region.bar().size()),
            isr_region.region(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR"));
    let device_config = unsafe {
        VolatileVirtioNetDeviceConfig::bind(
            mapped_pointer(device_region.bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR")),
            mapped_bytes(device_region.bar().size()),
            device_region.region(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR"));
    let rx_layout = VirtioNetQueueLayout::new(NET_RX_METADATA_IOVA, NET_RX_PACKET_IOVA)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_QUEUE_LAYOUT_ERROR"));
    let tx_layout = VirtioNetQueueLayout::new(NET_TX_METADATA_IOVA, NET_TX_PACKET_IOVA)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_QUEUE_LAYOUT_ERROR"));
    // SAFETY: the allocator returned four disjoint exclusive DMA frames.
    let mut net = VirtioNetDevice::bind(
        common,
        notify,
        isr,
        device_config,
        MMIO_POLL_BUDGET,
        unsafe { &mut *rx_metadata_pointer },
        unsafe { &mut *rx_packet_pointer },
        rx_layout,
        unsafe { &mut *tx_metadata_pointer },
        unsafe { &mut *tx_packet_pointer },
        tx_layout,
    )
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_BIND_ERROR"));

    hardware
        .configure_msix(smp_bootstrap.bsp_apic_id())
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MSIX_CONFIG_ERROR"));
    let mac = net
        .initialize(0, 1)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_INITIALIZATION_ERROR"));
    if mac != expected_mac {
        fatal_boot("AGENT_KERNEL_NATIVE_NET_MAC_MISMATCH_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_NET_MSIX_CONFIGURED_OK");

    let mut arp = [0; ARP_FRAME_BYTES];
    build_arp_request(&mut arp, mac);
    let transmit = authority
        .prepare_transmit(&mut booted, frame_descriptor(&arp))
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_TX_AUTHORITY_ERROR"));
    net.prepare_receive()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_RX_PREPARE_ERROR"));
    net.prepare_transmit(&arp)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_TX_PREPARE_ERROR"));
    hardware.activate_bus_master().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_BUS_MASTER_ERROR")
    });
    interrupts::reset();
    net.notify_receive().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_RX_NOTIFY_ERROR")
    });
    net.notify_transmit().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_TX_NOTIFY_ERROR")
    });

    if !interrupts::wait_for_counts(0, 1) {
        fatal_after_enable(
            &mut hardware,
            "AGENT_KERNEL_NATIVE_NET_TX_INTERRUPT_TIMEOUT_ERROR",
        );
    }
    net.complete_transmit().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_TX_COMPLETION_ERROR")
    });
    authority
        .complete_transmit(&mut booted, transmit)
        .unwrap_or_else(|_| {
            fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_TX_AUTHORITY_ERROR")
        });
    smp_bootstrap
        .complete_message_interrupt(true)
        .unwrap_or_else(|_| {
            fatal_after_enable(
                &mut hardware,
                "AGENT_KERNEL_NATIVE_NET_LOCAL_APIC_EOI_ERROR",
            )
        });
    serial_write_line("AGENT_KERNEL_NATIVE_NET_TX_MSIX_DELIVERED_OK");

    if !interrupts::wait_for_counts(1, 1) {
        fatal_after_enable(
            &mut hardware,
            "AGENT_KERNEL_NATIVE_NET_RX_INTERRUPT_TIMEOUT_ERROR",
        );
    }
    let received = net
        .complete_receive()
        .unwrap_or_else(|error| fatal_after_enable(&mut hardware, rx_error_marker(error)));
    let reply = net
        .frame(&received)
        .unwrap_or_else(|error| fatal_after_enable(&mut hardware, rx_error_marker(error)));
    if !is_expected_arp_reply(reply, mac) {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_ARP_REPLY_ERROR");
    }
    authority
        .record_receive(&mut booted, frame_descriptor(reply))
        .unwrap_or_else(|_| {
            fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_RX_AUTHORITY_ERROR")
        });
    smp_bootstrap
        .complete_message_interrupt(true)
        .unwrap_or_else(|_| {
            fatal_after_enable(
                &mut hardware,
                "AGENT_KERNEL_NATIVE_NET_LOCAL_APIC_EOI_ERROR",
            )
        });
    require_no_fault(&mut hardware, &mut iommu);
    serial_write_line("AGENT_KERNEL_NATIVE_NET_ARP_REPLY_OK");

    hardware.disable_msix().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_MSIX_DISABLE_ERROR")
    });
    net.shutdown().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_NATIVE_NET_SHUTDOWN_ERROR")
    });
    hardware
        .quiesce()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_QUIESCE_ERROR"));
    authority
        .begin_release(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_RELEASE_BEGIN_ERROR"));
    tables
        .detach_requester(hardware.requester(), domain)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_CONTEXT_REMOVE_ERROR"));
    publish_dma_memory();
    iommu
        .invalidate()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_INVALIDATION_ERROR"));
    authority
        .complete_release(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_RELEASE_COMPLETE_ERROR"));
    if !authority.released(&booted) || tables.active_requester_count() != 0 {
        fatal_boot("AGENT_KERNEL_NATIVE_NET_RELEASE_STATE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_NET_ENDPOINT_RELEASED_OK");

    let source_id = hardware.source_id();
    run_detached_dma_probe(
        &mut hardware,
        &mut net,
        &mut iommu,
        smp_bootstrap.bsp_apic_id(),
        source_id,
        &arp,
    );
    authority
        .begin_mapping_revoke(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MAPPING_REVOKE_ERROR"));
    for iova in [
        NET_RX_METADATA_IOVA,
        NET_RX_PACKET_IOVA,
        NET_TX_METADATA_IOVA,
        NET_TX_PACKET_IOVA,
    ] {
        tables
            .remove_mapping(iova)
            .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MAPPING_REMOVE_ERROR"));
    }
    publish_dma_memory();
    iommu
        .invalidate()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_INVALIDATION_ERROR"));
    authority
        .complete_mapping_revoke(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MAPPING_REVOKE_ERROR"));
    iommu
        .disable()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_IOMMU_DISABLE_ERROR"));
    serial_write_line("AGENT_KERNEL_NATIVE_NET_PROOF_OK");
    exit_qemu(0x10);
    halt_forever()
}
