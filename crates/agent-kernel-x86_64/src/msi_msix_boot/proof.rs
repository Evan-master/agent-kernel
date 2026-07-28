//! Bounded interrupt waits, DMA evidence checks, and emergency quiescence.
//!
//! These helpers keep volatile memory ordering and post-Bus-Master failure
//! handling outside the activation orchestrator. They own no Core state.

use core::{
    arch::asm,
    hint::spin_loop,
    sync::atomic::{compiler_fence, Ordering},
};

use agent_kernel_x86_64::{
    edu::{EduDma, EduRegisterIo, EDU_DEVICE_BUFFER, EDU_DMA_INTERRUPT},
    iommu::{IntelVtd, VtdControllerError, VtdFaultRecord, VtdRegisterIo},
};

use crate::{
    agent_memory::PHYSICAL_MEMORY_OFFSET, fatal_boot, serial_write_line, smp_boot::SmpBootstrap,
};

use super::{interrupts, pci, EDU_IOVA, FAULT_WAIT_SPINS, TRANSFER_BYTES};

pub(super) fn prove_edu_interrupt<I: VtdRegisterIo, E: EduRegisterIo>(
    hardware: &mut pci::PreparedMsiMsixHardware,
    bootstrap: &mut SmpBootstrap,
    iommu: &mut IntelVtd<I>,
    edu: &mut EduDma<E>,
    data: *mut u8,
    expected_edu: u8,
    expected_rng: u8,
) {
    write_pattern(data, expected_edu);
    publish_dma_memory();
    edu.copy_memory_to_device_interrupting(EDU_IOVA, EDU_DEVICE_BUFFER, TRANSFER_BYTES as u64)
        .unwrap_or_else(|_| fatal_after_enable(hardware, "AGENT_KERNEL_EDU_DMA_ERROR"));
    if !interrupts::wait_for_counts(expected_edu, expected_rng) {
        fatal_after_enable(hardware, "AGENT_KERNEL_EDU_MSI_TIMEOUT_ERROR");
    }
    let cause = edu
        .acknowledge_dma_interrupt()
        .unwrap_or_else(|_| fatal_after_enable(hardware, "AGENT_KERNEL_EDU_INTERRUPT_CAUSE_ERROR"));
    if cause != EDU_DMA_INTERRUPT {
        fatal_after_enable(hardware, "AGENT_KERNEL_EDU_INTERRUPT_CAUSE_ERROR");
    }
    bootstrap
        .complete_message_interrupt(true)
        .unwrap_or_else(|_| fatal_after_enable(hardware, "AGENT_KERNEL_LOCAL_APIC_EOI_ERROR"));
    require_no_fault(hardware, iommu);
}

pub(super) fn require_no_fault<I: VtdRegisterIo>(
    hardware: &mut pci::PreparedMsiMsixHardware,
    iommu: &mut IntelVtd<I>,
) {
    match iommu.fault_record() {
        Ok(None) => {}
        _ => fatal_after_enable(hardware, "AGENT_KERNEL_UNEXPECTED_DMA_FAULT_ERROR"),
    }
}

pub(super) fn wait_for_fault<I: VtdRegisterIo>(
    iommu: &mut IntelVtd<I>,
) -> Result<Option<VtdFaultRecord>, VtdControllerError> {
    for _ in 0..FAULT_WAIT_SPINS {
        if let Some(fault) = iommu.fault_record()? {
            return Ok(Some(fault));
        }
        spin_loop();
    }
    Ok(None)
}

pub(super) fn mapped_pointer(physical: u64) -> Option<*mut u8> {
    PHYSICAL_MEMORY_OFFSET
        .checked_add(physical)
        .map(|virtual_address| virtual_address as *mut u8)
}

pub(super) fn mapped_bytes(bytes: u64) -> usize {
    usize::try_from(bytes).unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_VIRTIO_RNG_MMIO_ERROR"))
}

pub(super) fn fatal_after_enable(hardware: &mut pci::PreparedMsiMsixHardware, marker: &str) -> ! {
    if hardware.quiesce_all().is_err() {
        serial_write_line(marker);
        fatal_boot("AGENT_KERNEL_MSI_MSIX_EMERGENCY_QUIESCE_ERROR");
    }
    fatal_boot(marker)
}

pub(super) fn fill(pointer: *mut u8, value: u8) {
    for index in 0..TRANSFER_BYTES {
        // SAFETY: the entropy frame remains exclusively owned by this proof.
        unsafe {
            pointer.add(index).write_volatile(value);
        }
    }
}

pub(super) fn all_equal(pointer: *mut u8, value: u8) -> bool {
    (0..TRANSFER_BYTES).all(|index| {
        // SAFETY: same exclusive frame contract as `fill`.
        unsafe { pointer.add(index).read_volatile() == value }
    })
}

pub(super) fn publish_dma_memory() {
    compiler_fence(Ordering::SeqCst);
    // SAFETY: MFENCE orders prior normal-memory writes before device MMIO.
    unsafe {
        asm!("mfence", options(nostack, preserves_flags));
    }
    compiler_fence(Ordering::SeqCst);
}

fn write_pattern(pointer: *mut u8, generation: u8) {
    for index in 0..TRANSFER_BYTES {
        // SAFETY: the DMA page owner supplies one exclusive 4 KiB page.
        unsafe {
            pointer.add(index).write_volatile(
                generation
                    .wrapping_mul(0x31)
                    .wrapping_add((index as u8).wrapping_mul(3)),
            );
        }
    }
}
