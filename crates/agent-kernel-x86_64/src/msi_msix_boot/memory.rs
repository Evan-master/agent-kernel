//! Exclusive VT-d, EDU, and Virtio queue pages for the V28 proof.
//!
//! Five frames hold translation tables. Three disjoint frames hold EDU data,
//! split-ring metadata, and entropy output. Every frame is removed from the
//! boot allocator before any device gains bus-master authority.

use agent_kernel_x86_64::iommu::{VtdLegacyTableAddresses, VtdLegacyTablePages, VtdTableError};
use bootloader_api::BootInfo;
use x86_64::{structures::paging::PhysFrame, PhysAddr};

use crate::agent_memory::{BootFrameAllocator, PHYSICAL_MEMORY_OFFSET};

const PAGE_BYTES: usize = 4096;
const FRAME_COUNT: usize = 8;
const EDU_FRAME: usize = 5;
const QUEUE_FRAME: usize = 6;
const ENTROPY_FRAME: usize = 7;

pub(super) struct NativeMsiMsixPages {
    frames: [u64; FRAME_COUNT],
}

impl NativeMsiMsixPages {
    pub(super) const fn edu_physical(&self) -> u64 {
        self.frames[EDU_FRAME]
    }

    pub(super) const fn queue_physical(&self) -> u64 {
        self.frames[QUEUE_FRAME]
    }

    pub(super) const fn entropy_physical(&self) -> u64 {
        self.frames[ENTROPY_FRAME]
    }

    pub(super) fn edu_pointer(&self) -> *mut u8 {
        physical_pointer(self.edu_physical())
    }

    pub(super) fn queue_pointer(&self) -> *mut [u8; PAGE_BYTES] {
        physical_pointer(self.queue_physical()).cast()
    }

    pub(super) fn entropy_pointer(&self) -> *mut [u8; PAGE_BYTES] {
        physical_pointer(self.entropy_physical()).cast()
    }

    pub(super) fn table_pages(&mut self) -> Result<VtdLegacyTablePages<'_>, VtdTableError> {
        let addresses = VtdLegacyTableAddresses::new(
            self.frames[0],
            self.frames[1],
            self.frames[2],
            self.frames[3],
            self.frames[4],
        )?;
        // SAFETY: allocation removed eight distinct frames from BootInfo. The
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

pub(super) fn allocate(boot_info: &mut BootInfo) -> Option<NativeMsiMsixPages> {
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
        // SAFETY: each frame was removed from a Usable region and is now
        // exclusively owned by this boot proof.
        unsafe {
            core::ptr::write_bytes(pointer, 0, PAGE_BYTES);
        }
    }
    Some(NativeMsiMsixPages { frames })
}

fn physical_pointer(physical: u64) -> *mut u8 {
    (PHYSICAL_MEMORY_OFFSET + physical) as *mut u8
}

unsafe fn table_pointer(physical: u64) -> &'static mut [u64; 512] {
    // SAFETY: caller guarantees one exclusive aligned table frame.
    unsafe { &mut *physical_pointer(physical).cast::<[u64; 512]>() }
}
