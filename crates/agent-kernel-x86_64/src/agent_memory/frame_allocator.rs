//! Bounded physical-frame allocation from bootloader Usable regions.
//!
//! This child module advances region starts one page at a time, ensuring every
//! frame returned to the Agent mapper is exclusive and never repeated.

use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

use agent_kernel_x86_64::user_memory::PAGE_BYTES;

pub(super) struct BootFrameAllocator<'a> {
    regions: &'a mut [MemoryRegion],
}

impl<'a> BootFrameAllocator<'a> {
    pub(super) fn new(regions: &'a mut [MemoryRegion]) -> Self {
        Self { regions }
    }
}

// SAFETY: each returned frame is removed from the front of one Usable region,
// so this allocator never returns the same frame twice.
unsafe impl FrameAllocator<Size4KiB> for BootFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        for region in &mut *self.regions {
            if region.kind != MemoryRegionKind::Usable {
                continue;
            }
            let Some(start) = align_up(region.start, PAGE_BYTES) else {
                continue;
            };
            let Some(end) = start.checked_add(PAGE_BYTES) else {
                continue;
            };
            if end > region.end {
                continue;
            }
            region.start = end;
            return PhysFrame::from_start_address(PhysAddr::new(start)).ok();
        }
        None
    }
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    value
        .checked_add(alignment.checked_sub(1)?)?
        .checked_div(alignment)?
        .checked_mul(alignment)
}
