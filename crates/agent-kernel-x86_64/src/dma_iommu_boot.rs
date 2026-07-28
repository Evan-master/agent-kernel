//! Native QEMU DMA/IOMMU closed-loop proof.
//!
//! This dedicated boot profile connects Core DMA Capabilities to QEMU EDU and
//! Intel VT-d before SMP or legacy PCI interrupt setup. The default boot flow
//! remains independent of Q35-specific hardware.

mod authority;
mod memory;
mod pci;

use core::{
    arch::asm,
    sync::atomic::{compiler_fence, Ordering},
};

use agent_kernel_core::DmaAccess;
use agent_kernel_x86_64::{
    edu::{EduDma, VolatileEduMmio, EDU_DEVICE_BUFFER},
    iommu::{IntelVtd, VolatileVtdMmio, VtdDomainId},
};
use bootloader_api::BootInfo;

use crate::{
    exit_qemu, fatal_boot, halt_forever, privilege_runtime::PrivilegeBoundary, serial_write_line,
    smp_boot::SmpBootstrap,
};

const IOVA: u64 = 0x0100_0000;
const TRANSFER_BYTES: usize = 64;
const MMIO_POLL_BUDGET: u32 = 100_000_000;

pub(super) fn run(
    boot_info: &'static mut BootInfo,
    privilege_boundary: PrivilegeBoundary,
    mut smp_bootstrap: SmpBootstrap,
) -> ! {
    let _privilege_boundary = privilege_boundary;
    let mut hardware = pci::prepare(&mut smp_bootstrap, boot_info)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_PCI_PREPARATION_ERROR"));
    serial_write_line("AGENT_KERNEL_DMAR_DISCOVERY_OK");
    serial_write_line("AGENT_KERNEL_EDU_PCI_TARGET_OK");
    serial_write_line("AGENT_KERNEL_DMA_BUS_MASTER_QUIESCED_OK");

    let (mut booted, binding) = authority::reserve(hardware.source_id(), IOVA)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_AUTHORITY_ERROR"));
    serial_write_line("AGENT_KERNEL_DMA_CAPABILITY_OK");

    let mut pages = memory::allocate(boot_info)
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_DMA_FRAME_ALLOCATION_ERROR"));
    let data_pointer = pages.data_pointer();
    let data_physical = pages.data_physical();
    let mut tables = pages.table_pages().unwrap_or_else(|_| {
        fatal_boot("AGENT_KERNEL_DMA_TRANSLATION_TABLE_ERROR");
    });
    tables
        .install(
            hardware.requester(),
            VtdDomainId::new(1).expect("fixed nonzero VT-d domain"),
            IOVA,
            data_physical,
            DmaAccess::ReadWrite,
        )
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_TRANSLATION_TABLE_ERROR"));
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
    authority::activate(&mut booted, binding)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_AUTHORITY_ACTIVATION_ERROR"));
    if hardware.enable().is_err() {
        fatal_after_dma_enable(&mut hardware, "AGENT_KERNEL_DMA_BUS_MASTER_ENABLE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_VTD_TRANSLATION_OK");

    let edu_pointer = mapped_pointer(hardware.edu_base());
    let edu_pointer = option_or_quiesce(
        &mut hardware,
        edu_pointer,
        "AGENT_KERNEL_EDU_MMIO_POINTER_ERROR",
    );
    // SAFETY: PCI preparation mapped EDU BAR0's register page uncached.
    let edu_io = unsafe { VolatileEduMmio::new(edu_pointer) };
    let edu_io = option_or_quiesce(&mut hardware, edu_io, "AGENT_KERNEL_EDU_MMIO_POINTER_ERROR");
    let edu = EduDma::bind(edu_io, MMIO_POLL_BUDGET);
    let mut edu = result_or_quiesce(&mut hardware, edu, "AGENT_KERNEL_EDU_BIND_ERROR");

    write_pattern(data_pointer);
    publish_dma_memory();
    let allowed_upload = edu.copy_memory_to_device(IOVA, EDU_DEVICE_BUFFER, TRANSFER_BYTES as u64);
    result_or_quiesce(
        &mut hardware,
        allowed_upload,
        "AGENT_KERNEL_DMA_ALLOWED_TRANSFER_ERROR",
    );
    let allowed_fault = iommu.fault_record();
    if result_or_quiesce(
        &mut hardware,
        allowed_fault,
        "AGENT_KERNEL_IOMMU_FAULT_READ_ERROR",
    )
    .is_some()
    {
        fatal_after_dma_enable(&mut hardware, "AGENT_KERNEL_DMA_ALLOWED_FAULT_ERROR");
    }
    fill(data_pointer, 0);
    publish_dma_memory();
    let allowed_download =
        edu.copy_device_to_memory(EDU_DEVICE_BUFFER, IOVA, TRANSFER_BYTES as u64);
    result_or_quiesce(
        &mut hardware,
        allowed_download,
        "AGENT_KERNEL_DMA_ALLOWED_TRANSFER_ERROR",
    );
    publish_dma_memory();
    if !matches_pattern(data_pointer) {
        fatal_after_dma_enable(&mut hardware, "AGENT_KERNEL_DMA_ALLOWED_DATA_ERROR");
    }
    serial_write_line("AGENT_KERNEL_DMA_ALLOWED_OK");

    let revoke = authority::begin_revoke(&mut booted, binding);
    result_or_quiesce(&mut hardware, revoke, "AGENT_KERNEL_DMA_REVOKE_BEGIN_ERROR");
    let removal = tables.remove(IOVA);
    result_or_quiesce(
        &mut hardware,
        removal,
        "AGENT_KERNEL_DMA_TRANSLATION_REMOVE_ERROR",
    );
    publish_dma_memory();
    let invalidation = iommu.invalidate();
    result_or_quiesce(
        &mut hardware,
        invalidation,
        "AGENT_KERNEL_DMA_INVALIDATION_ERROR",
    );
    let release = authority::complete_revoke(&mut booted, binding);
    result_or_quiesce(
        &mut hardware,
        release,
        "AGENT_KERNEL_DMA_REVOKE_COMPLETE_ERROR",
    );

    fill(data_pointer, 0xa5);
    publish_dma_memory();
    let blocked_transfer =
        edu.copy_device_to_memory(EDU_DEVICE_BUFFER, IOVA, TRANSFER_BYTES as u64);
    result_or_quiesce(
        &mut hardware,
        blocked_transfer,
        "AGENT_KERNEL_DMA_BLOCKED_TRANSFER_ERROR",
    );
    publish_dma_memory();
    if !all_equal(data_pointer, 0xa5) {
        fatal_after_dma_enable(&mut hardware, "AGENT_KERNEL_DMA_BLOCKED_MUTATION_ERROR");
    }
    let fault = iommu.fault_record();
    let fault = result_or_quiesce(&mut hardware, fault, "AGENT_KERNEL_IOMMU_FAULT_READ_ERROR");
    let fault = option_or_quiesce(
        &mut hardware,
        fault,
        "AGENT_KERNEL_DMA_BLOCKED_FAULT_MISSING_ERROR",
    );
    if fault.source_id() != hardware.source_id()
        || fault.address() != IOVA
        || fault.reason() != 5
        || !fault.write()
    {
        fatal_after_dma_enable(
            &mut hardware,
            "AGENT_KERNEL_DMA_BLOCKED_FAULT_MISMATCH_ERROR",
        );
    }
    let fault_clear = iommu.clear_fault();
    result_or_quiesce(
        &mut hardware,
        fault_clear,
        "AGENT_KERNEL_IOMMU_FAULT_CLEAR_ERROR",
    );
    serial_write_line("AGENT_KERNEL_DMA_REVOKED_FAULT_OK");

    hardware
        .quiesce()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_DMA_BUS_MASTER_DISABLE_ERROR"));
    iommu
        .disable()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_IOMMU_DISABLE_ERROR"));
    serial_write_line("AGENT_KERNEL_DMA_IOMMU_PROOF_OK");
    exit_qemu(0x10);
    halt_forever()
}

fn mapped_pointer(physical: u64) -> Option<*mut u8> {
    crate::agent_memory::PHYSICAL_MEMORY_OFFSET
        .checked_add(physical)
        .map(|virtual_address| virtual_address as *mut u8)
}

fn result_or_quiesce<T, E>(
    hardware: &mut pci::PreparedDmaHardware,
    result: Result<T, E>,
    marker: &str,
) -> T {
    match result {
        Ok(value) => value,
        Err(_) => fatal_after_dma_enable(hardware, marker),
    }
}

fn option_or_quiesce<T>(
    hardware: &mut pci::PreparedDmaHardware,
    value: Option<T>,
    marker: &str,
) -> T {
    match value {
        Some(value) => value,
        None => fatal_after_dma_enable(hardware, marker),
    }
}

fn fatal_after_dma_enable(hardware: &mut pci::PreparedDmaHardware, marker: &str) -> ! {
    if hardware.quiesce().is_err() {
        serial_write_line(marker);
        fatal_boot("AGENT_KERNEL_DMA_EMERGENCY_QUIESCE_ERROR");
    }
    fatal_boot(marker)
}

fn write_pattern(pointer: *mut u8) {
    for index in 0..TRANSFER_BYTES {
        // SAFETY: the DMA page owner supplies a complete exclusive 4 KiB page.
        unsafe {
            pointer
                .add(index)
                .write_volatile(0x21_u8.wrapping_add((index as u8).wrapping_mul(3)));
        }
    }
}

fn matches_pattern(pointer: *mut u8) -> bool {
    (0..TRANSFER_BYTES).all(|index| {
        // SAFETY: same exclusive DMA page contract as `write_pattern`.
        (unsafe { pointer.add(index).read_volatile() })
            == 0x21_u8.wrapping_add((index as u8).wrapping_mul(3))
    })
}

fn fill(pointer: *mut u8, value: u8) {
    for index in 0..TRANSFER_BYTES {
        // SAFETY: same exclusive DMA page contract as `write_pattern`.
        unsafe {
            pointer.add(index).write_volatile(value);
        }
    }
}

fn all_equal(pointer: *mut u8, value: u8) -> bool {
    (0..TRANSFER_BYTES).all(|index| {
        // SAFETY: same exclusive DMA page contract as `write_pattern`.
        unsafe { pointer.add(index).read_volatile() == value }
    })
}

fn publish_dma_memory() {
    compiler_fence(Ordering::SeqCst);
    // SAFETY: `mfence` only orders the BSP's prior normal-memory writes before
    // subsequent uncached MMIO operations.
    unsafe {
        asm!("mfence", options(nostack, preserves_flags));
    }
    compiler_fence(Ordering::SeqCst);
}
