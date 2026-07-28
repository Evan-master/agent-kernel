//! Native QEMU MSI/MSI-X and shared-domain closed-loop proof.
//!
//! This dedicated boot profile composes Core authority, PCI message
//! capabilities, Local APIC delivery, modern Virtio RNG, and two-requester
//! VT-d isolation. The default and V27 boot profiles remain independent.

mod authority;
mod interrupts;
mod memory;
mod pci;
mod proof;

use agent_kernel_core::DmaAccess;
use agent_kernel_x86_64::{
    edu::{EduDma, VolatileEduMmio},
    iommu::{IntelVtd, VolatileVtdMmio, VtdDomainId},
    virtio_rng::{
        VirtioRngDevice, VirtioRngQueueLayout, VolatileVirtioCommonConfig, VolatileVirtioIsr,
        VolatileVirtioNotify,
    },
};
use bootloader_api::BootInfo;

use crate::{
    exception_runtime, exit_qemu, fatal_boot, halt_forever, privilege_runtime::PrivilegeBoundary,
    serial_write_line, smp_boot::SmpBootstrap,
};

use self::proof::{
    all_equal, fatal_after_enable, fill, mapped_bytes, mapped_pointer, prove_edu_interrupt,
    publish_dma_memory, require_no_fault, wait_for_fault,
};

pub(super) const EDU_IOVA: u64 = 0x0100_0000;
pub(super) const RNG_QUEUE_IOVA: u64 = 0x0100_1000;
pub(super) const RNG_ENTROPY_IOVA: u64 = 0x0100_2000;
pub(super) const EDU_MSI_VECTOR: u8 = 0xd0;
pub(super) const RNG_MSIX_VECTOR: u8 = 0xd1;

const TRANSFER_BYTES: usize = 64;
const MMIO_POLL_BUDGET: u32 = 100_000_000;
const FAULT_WAIT_SPINS: usize = 100_000_000;

pub(super) fn run(
    boot_info: &'static mut BootInfo,
    privilege_boundary: PrivilegeBoundary,
    mut smp_bootstrap: SmpBootstrap,
) -> ! {
    let _privilege_boundary = privilege_boundary;
    interrupts::install_gates().unwrap_or_else(|| fatal_boot("AGENT_KERNEL_MSI_MSIX_IDT_ERROR"));
    smp_bootstrap
        .prepare_apic_mmio(boot_info)
        .unwrap_or_else(|error| fatal_boot(error.diagnostic_marker()));
    exception_runtime::freeze_for_smp()
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_MSI_MSIX_IDT_FREEZE_ERROR"));

    let mut hardware = pci::prepare(&mut smp_bootstrap, boot_info)
        .unwrap_or_else(|error| fatal_boot(error.diagnostic_marker()));
    serial_write_line("AGENT_KERNEL_DMAR_DISCOVERY_OK");

    let (mut booted, authority) = authority::reserve(
        smp_bootstrap.bsp_apic_id().get(),
        hardware.edu_source_id(),
        hardware.rng_source_id(),
    )
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_AUTHORITY_ERROR"));
    serial_write_line("AGENT_KERNEL_INTERRUPT_CAPABILITY_OK");

    let mut pages = memory::allocate(boot_info)
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_MSI_MSIX_FRAME_ALLOCATION_ERROR"));
    let edu_pointer = pages.edu_pointer();
    let queue_pointer = pages.queue_pointer();
    let entropy_pointer = pages.entropy_pointer();
    let edu_physical = pages.edu_physical();
    let queue_physical = pages.queue_physical();
    let entropy_physical = pages.entropy_physical();
    let mut tables = pages
        .table_pages()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_TABLE_ERROR"));
    let domain = VtdDomainId::new(1).expect("fixed nonzero VT-d domain");
    tables
        .attach_requester(hardware.edu_requester(), domain)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_TABLE_ERROR"));
    tables
        .attach_requester(hardware.rng_requester(), domain)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_TABLE_ERROR"));
    tables
        .install_mapping(EDU_IOVA, edu_physical, DmaAccess::ReadWrite)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_TABLE_ERROR"));
    tables
        .install_mapping(RNG_QUEUE_IOVA, queue_physical, DmaAccess::ReadWrite)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_TABLE_ERROR"));
    tables
        .install_mapping(RNG_ENTROPY_IOVA, entropy_physical, DmaAccess::Write)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_TABLE_ERROR"));
    publish_dma_memory();

    let iommu_pointer = mapped_pointer(hardware.iommu_base())
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_IOMMU_MMIO_POINTER_ERROR"));
    // SAFETY: PCI preparation mapped the complete DRHD register page uncached.
    let iommu_io = unsafe { VolatileVtdMmio::new(iommu_pointer) }
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_IOMMU_MMIO_POINTER_ERROR"));
    let mut iommu = IntelVtd::bind(iommu_io, MMIO_POLL_BUDGET)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_IOMMU_BIND_ERROR"));
    iommu
        .activate(tables.root_address())
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_IOMMU_ACTIVATION_ERROR"));
    authority
        .activate(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_AUTHORITY_ACTIVATION_ERROR"));
    if !authority.has_two_active_attachments(&booted) {
        fatal_boot("AGENT_KERNEL_MULTI_DEVICE_DMA_DOMAIN_ERROR");
    }
    serial_write_line("AGENT_KERNEL_MULTI_DEVICE_DMA_DOMAIN_OK");

    hardware
        .enable_memory_decode()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_MEMORY_DECODE_ERROR"));
    let edu_mmio = mapped_pointer(hardware.edu_base())
        .and_then(|pointer| unsafe { VolatileEduMmio::new(pointer) })
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_EDU_MMIO_POINTER_ERROR"));
    let mut edu = EduDma::bind(edu_mmio, MMIO_POLL_BUDGET)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_EDU_BIND_ERROR"));

    let regions = hardware.rng_regions();
    let common_region = regions.common();
    let notify_region = regions.notify();
    let isr_region = regions.isr();
    // SAFETY: the three BAR ranges were validated and mapped uncached during
    // PCI preparation, and this proof retains exclusive function ownership.
    let common = unsafe {
        VolatileVirtioCommonConfig::bind(
            mapped_pointer(common_region.bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_MMIO_ERROR")),
            mapped_bytes(common_region.bar().size()),
            common_region.region(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_MMIO_ERROR"));
    let notify = unsafe {
        VolatileVirtioNotify::bind(
            mapped_pointer(notify_region.bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_MMIO_ERROR")),
            mapped_bytes(notify_region.bar().size()),
            notify_region.region(),
            hardware.notify_multiplier(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_MMIO_ERROR"));
    let isr = unsafe {
        VolatileVirtioIsr::bind(
            mapped_pointer(isr_region.bar().base())
                .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_MMIO_ERROR")),
            mapped_bytes(isr_region.bar().size()),
            isr_region.region(),
        )
    }
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_MMIO_ERROR"));
    let layout = VirtioRngQueueLayout::new(RNG_QUEUE_IOVA, RNG_ENTROPY_IOVA)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_QUEUE_ERROR"));
    // SAFETY: the allocator returned disjoint exclusive queue and entropy
    // frames, and the VT-d table pages borrow only the first five frames.
    let metadata = unsafe { &mut *queue_pointer };
    let entropy = unsafe { &mut *entropy_pointer };
    let mut rng = VirtioRngDevice::bind(
        common,
        notify,
        isr,
        MMIO_POLL_BUDGET,
        metadata,
        entropy,
        layout,
    )
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_BIND_ERROR"));

    hardware
        .configure_edu_msi(smp_bootstrap.bsp_apic_id())
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_CONFIGURATION_ERROR"));
    serial_write_line("AGENT_KERNEL_MSI_CONFIGURED_OK");
    hardware
        .configure_rng_msix(smp_bootstrap.bsp_apic_id())
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSIX_CONFIGURATION_ERROR"));
    rng.initialize(0)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_INITIALIZATION_ERROR"));
    serial_write_line("AGENT_KERNEL_MSIX_CONFIGURED_OK");
    hardware
        .activate_bus_master()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_BUS_MASTER_ERROR"));

    interrupts::reset();
    prove_edu_interrupt(
        &mut hardware,
        &mut smp_bootstrap,
        &mut iommu,
        &mut edu,
        edu_pointer,
        1,
        0,
    );
    serial_write_line("AGENT_KERNEL_EDU_MSI_DELIVERED_OK");

    rng.request_entropy(TRANSFER_BYTES as u32)
        .unwrap_or_else(|_| {
            fatal_after_enable(&mut hardware, "AGENT_KERNEL_VIRTIO_RNG_REQUEST_ERROR")
        });
    if !interrupts::wait_for_counts(1, 1) {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_VIRTIO_RNG_MSIX_TIMEOUT_ERROR");
    }
    let completion = rng.complete_interrupt().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_VIRTIO_RNG_COMPLETION_ERROR")
    });
    smp_bootstrap
        .complete_message_interrupt(true)
        .unwrap_or_else(|_| fatal_after_enable(&mut hardware, "AGENT_KERNEL_LOCAL_APIC_EOI_ERROR"));
    if completion.len() != TRANSFER_BYTES as u32
        || rng.entropy(&completion).iter().all(|byte| *byte == 0)
    {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_VIRTIO_RNG_DATA_ERROR");
    }
    require_no_fault(&mut hardware, &mut iommu);
    serial_write_line("AGENT_KERNEL_VIRTIO_RNG_MSIX_DELIVERED_OK");

    hardware
        .disable_rng_msix()
        .unwrap_or_else(|_| fatal_after_enable(&mut hardware, "AGENT_KERNEL_MSIX_DISABLE_ERROR"));
    rng.shutdown().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_VIRTIO_RNG_SHUTDOWN_ERROR")
    });
    hardware.quiesce_rng().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_VIRTIO_RNG_QUIESCE_ERROR")
    });
    authority.begin_rng_detach(&mut booted).unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DETACH_BEGIN_ERROR")
    });
    tables
        .detach_requester(hardware.rng_requester(), domain)
        .unwrap_or_else(|_| {
            fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_CONTEXT_REMOVE_ERROR")
        });
    publish_dma_memory();
    iommu.invalidate().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_INVALIDATION_ERROR")
    });
    authority
        .complete_rng_detach(&mut booted)
        .unwrap_or_else(|_| {
            fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DETACH_COMPLETE_ERROR")
        });
    if !authority.rng_detached_with_edu_survivor(&booted) || tables.active_requester_count() != 1 {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DETACH_STATE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_DMA_REQUESTER_DETACHED_OK");

    hardware.enable_rng_memory_decode().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DENIAL_PROBE_ERROR")
    });
    rng.initialize(0).unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DENIAL_PROBE_ERROR")
    });
    rng.prepare_entropy_request(TRANSFER_BYTES as u32)
        .unwrap_or_else(|_| {
            fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DENIAL_PROBE_ERROR")
        });
    fill(entropy_pointer.cast::<u8>(), 0xa5);
    publish_dma_memory();
    hardware.enable_rng_bus_master().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DENIAL_PROBE_ERROR")
    });
    rng.notify_entropy_request().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DENIAL_PROBE_ERROR")
    });
    let fault = wait_for_fault(&mut iommu)
        .unwrap_or_else(|_| {
            fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DETACH_FAULT_READ_ERROR")
        })
        .unwrap_or_else(|| {
            fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DETACH_FAULT_MISSING_ERROR")
        });
    if fault.source_id() != hardware.rng_source_id()
        || (fault.address() != RNG_QUEUE_IOVA && fault.address() != RNG_ENTROPY_IOVA)
        || !all_equal(entropy_pointer.cast::<u8>(), 0xa5)
        || interrupts::current_counts() != (1, 1)
    {
        fatal_after_enable(
            &mut hardware,
            "AGENT_KERNEL_DMA_DETACH_FAULT_MISMATCH_ERROR",
        );
    }
    rng.shutdown().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DENIAL_PROBE_ERROR")
    });
    hardware.quiesce_rng().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DENIAL_PROBE_ERROR")
    });
    iommu.clear_fault().unwrap_or_else(|_| {
        fatal_after_enable(&mut hardware, "AGENT_KERNEL_DMA_DETACH_FAULT_CLEAR_ERROR")
    });
    serial_write_line("AGENT_KERNEL_DMA_DETACH_FAULT_OK");

    prove_edu_interrupt(
        &mut hardware,
        &mut smp_bootstrap,
        &mut iommu,
        &mut edu,
        edu_pointer,
        2,
        1,
    );
    serial_write_line("AGENT_KERNEL_SHARED_DOMAIN_SURVIVOR_OK");

    hardware
        .disable_edu_msi()
        .unwrap_or_else(|_| fatal_after_enable(&mut hardware, "AGENT_KERNEL_MSI_DISABLE_ERROR"));
    hardware
        .quiesce_all()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_MSI_MSIX_QUIESCE_ERROR"));
    authority
        .begin_edu_detach(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_EDU_DETACH_BEGIN_ERROR"));
    tables
        .detach_requester(hardware.edu_requester(), domain)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_EDU_CONTEXT_REMOVE_ERROR"));
    publish_dma_memory();
    iommu
        .invalidate()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_INVALIDATION_ERROR"));
    authority
        .complete_edu_detach(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_EDU_DETACH_COMPLETE_ERROR"));
    authority
        .begin_mapping_revoke(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_MAPPING_REVOKE_ERROR"));
    for iova in [EDU_IOVA, RNG_QUEUE_IOVA, RNG_ENTROPY_IOVA] {
        tables
            .remove_mapping(iova)
            .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_MAPPING_REMOVE_ERROR"));
    }
    publish_dma_memory();
    iommu
        .invalidate()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_INVALIDATION_ERROR"));
    authority
        .complete_mapping_revoke(&mut booted)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_MAPPING_REVOKE_ERROR"));
    if !authority.routes_released(&booted) {
        fatal_boot("AGENT_KERNEL_INTERRUPT_ROUTE_RELEASE_ERROR");
    }
    iommu
        .disable()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_IOMMU_DISABLE_ERROR"));
    serial_write_line("AGENT_KERNEL_MSI_MSIX_PROOF_OK");
    exit_qemu(0x10);
    halt_forever()
}
