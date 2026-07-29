//! DMA ordering, VT-d fault waits, and emergency network quiescence.
//!
//! These helpers own post-Bus-Master failure handling and no Core state.

use core::{
    arch::asm,
    hint::spin_loop,
    sync::atomic::{compiler_fence, Ordering},
};

use agent_kernel_x86_64::iommu::{IntelVtd, VtdControllerError, VtdFaultRecord, VtdRegisterIo};

use crate::{agent_memory::PHYSICAL_MEMORY_OFFSET, fatal_boot, serial_write_line};

use super::{pci, FAULT_WAIT_SPINS};

pub(super) fn require_no_fault<I: VtdRegisterIo>(
    hardware: &mut pci::PreparedNativeNetHardware,
    iommu: &mut IntelVtd<I>,
) {
    match iommu.fault_record() {
        Ok(None) => {}
        _ => fatal_after_enable(
            hardware,
            "AGENT_KERNEL_NATIVE_NET_UNEXPECTED_DMA_FAULT_ERROR",
        ),
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
    usize::try_from(bytes)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_MMIO_POINTER_ERROR"))
}

pub(super) fn fatal_after_enable(hardware: &mut pci::PreparedNativeNetHardware, marker: &str) -> ! {
    if hardware.quiesce().is_err() {
        serial_write_line(marker);
        fatal_boot("AGENT_KERNEL_NATIVE_NET_EMERGENCY_QUIESCE_ERROR");
    }
    fatal_boot(marker)
}

pub(super) fn publish_dma_memory() {
    compiler_fence(Ordering::SeqCst);
    // SAFETY: MFENCE orders normal-memory writes before subsequent device MMIO.
    unsafe {
        asm!("mfence", options(nostack, preserves_flags));
    }
    compiler_fence(Ordering::SeqCst);
}
