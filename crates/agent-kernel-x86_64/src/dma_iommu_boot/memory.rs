//! Exclusive physical pages for the native DMA proof.

use agent_kernel_x86_64::iommu::{VtdLegacyTableAddresses, VtdLegacyTablePages, VtdTableError};
use bootloader_api::BootInfo;
use x86_64::{structures::paging::PhysFrame, PhysAddr};

use crate::agent_memory::{BootFrameAllocator, PHYSICAL_MEMORY_OFFSET};

const PAGE_BYTES: usize = 4096;
const FRAME_COUNT: usize = 6;

pub(super) struct NativeDmaPages {
    frames: [u64; FRAME_COUNT],
}

impl NativeDmaPages {
    pub(super) fn data_physical(&self) -> u64 {
        self.frames[5]
    }

    pub(super) fn data_pointer(&self) -> *mut u8 {
        physical_pointer(self.frames[5])
    }

    pub(super) fn table_pages(&mut self) -> Result<VtdLegacyTablePages<'_>, VtdTableError> {
        let addresses = VtdLegacyTableAddresses::new(
            self.frames[0],
            self.frames[1],
            self.frames[2],
            self.frames[3],
            self.frames[4],
        )?;
        // SAFETY: allocation removed six distinct frames from BootInfo. The
        // first five remain exclusively owned by this table set.
        unsafe {
            Ok(VtdLegacyTablePages::new(
                table_pointer(self.frames[0]),
                table_pointer(self.frames[1]),
                table_pointer(self.frames[2]),
                table_pointer(self.frames[3]),
                table_pointer(self.frames[4]),
                addresses,
            ))
        }
    }
}

pub(super) fn allocate(boot_info: &mut BootInfo) -> Option<NativeDmaPages> {
    if boot_info.physical_memory_offset.into_option()? != PHYSICAL_MEMORY_OFFSET {
        return None;
    }
    let zero = PhysFrame::from_start_address(PhysAddr::new(0)).ok()?;
    let mut allocated = [zero; FRAME_COUNT];
    let mut allocator = BootFrameAllocator::new(&mut boot_info.memory_regions);
    for frame in &mut allocated {
        *frame = allocator.allocate()?;
    }
    let mut frames = [0_u64; FRAME_COUNT];
    for (index, frame) in allocated.iter().enumerate() {
        frames[index] = frame.start_address().as_u64();
        let pointer = physical_pointer(frames[index]);
        // SAFETY: each frame was removed from a Usable region and is exclusively
        // owned by this proof.
        unsafe {
            core::ptr::write_bytes(pointer, 0, PAGE_BYTES);
        }
    }
    Some(NativeDmaPages { frames })
}

fn physical_pointer(physical: u64) -> *mut u8 {
    (PHYSICAL_MEMORY_OFFSET + physical) as *mut u8
}

unsafe fn table_pointer(physical: u64) -> &'static mut [u64; 512] {
    // SAFETY: caller guarantees one exclusive aligned table frame.
    unsafe { &mut *physical_pointer(physical).cast::<[u64; 512]>() }
}
