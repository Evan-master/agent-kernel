//! Allocation tracking for one private x86 Agent page-table hierarchy.
//!
//! This bare-metal page-table child wraps the boot allocator while the mapper
//! creates P3, P2, and P1 tables below the explicit P4 root. It rejects any
//! allocation count outside the fixed single-slot hierarchy.

use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};

use agent_kernel_x86_64::address_space::AGENT_PAGE_TABLE_FRAME_COUNT;

use super::super::BootFrameAllocator;

const INTERMEDIATE_FRAME_COUNT: usize = AGENT_PAGE_TABLE_FRAME_COUNT - 1;

pub(super) struct TrackedPageTableAllocator<'allocator, 'regions> {
    allocator: &'allocator mut BootFrameAllocator<'regions>,
    frames: [Option<PhysFrame<Size4KiB>>; INTERMEDIATE_FRAME_COUNT],
    len: usize,
}

impl<'allocator, 'regions> TrackedPageTableAllocator<'allocator, 'regions> {
    pub(super) fn new(allocator: &'allocator mut BootFrameAllocator<'regions>) -> Self {
        Self {
            allocator,
            frames: [None; INTERMEDIATE_FRAME_COUNT],
            len: 0,
        }
    }

    pub(super) fn finish(
        self,
        root: PhysFrame<Size4KiB>,
    ) -> Option<[u64; AGENT_PAGE_TABLE_FRAME_COUNT]> {
        if self.len != INTERMEDIATE_FRAME_COUNT {
            return None;
        }
        let mut result = [0; AGENT_PAGE_TABLE_FRAME_COUNT];
        result[0] = root.start_address().as_u64();
        for (index, frame) in self.frames.into_iter().enumerate() {
            result[index + 1] = frame?.start_address().as_u64();
        }
        Some(result)
    }
}

// SAFETY: the wrapped allocator transfers each frame exactly once. This layer
// records that transfer and refuses a fourth private intermediate table.
unsafe impl FrameAllocator<Size4KiB> for TrackedPageTableAllocator<'_, '_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if self.len >= INTERMEDIATE_FRAME_COUNT {
            return None;
        }
        let frame = self.allocator.allocate()?;
        self.frames[self.len] = Some(frame);
        self.len += 1;
        Some(frame)
    }
}
