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

pub(super) struct ReusedPageTableAllocator {
    frames: [Option<PhysFrame<Size4KiB>>; INTERMEDIATE_FRAME_COUNT],
    len: usize,
}

pub(super) trait PrivatePageTableAllocator: FrameAllocator<Size4KiB> {
    fn finish(self, root: PhysFrame<Size4KiB>) -> Option<[u64; AGENT_PAGE_TABLE_FRAME_COUNT]>;
}

impl<'allocator, 'regions> TrackedPageTableAllocator<'allocator, 'regions> {
    pub(super) fn new(allocator: &'allocator mut BootFrameAllocator<'regions>) -> Self {
        Self {
            allocator,
            frames: [None; INTERMEDIATE_FRAME_COUNT],
            len: 0,
        }
    }
}

impl PrivatePageTableAllocator for TrackedPageTableAllocator<'_, '_> {
    fn finish(self, root: PhysFrame<Size4KiB>) -> Option<[u64; AGENT_PAGE_TABLE_FRAME_COUNT]> {
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

impl ReusedPageTableAllocator {
    pub(super) fn new(
        frames: [u64; AGENT_PAGE_TABLE_FRAME_COUNT],
    ) -> Option<(PhysFrame<Size4KiB>, Self)> {
        let root = PhysFrame::from_start_address(x86_64::PhysAddr::new(frames[0])).ok()?;
        let mut intermediates = [None; INTERMEDIATE_FRAME_COUNT];
        for (slot, address) in intermediates.iter_mut().zip(frames[1..].iter().copied()) {
            *slot = Some(PhysFrame::from_start_address(x86_64::PhysAddr::new(address)).ok()?);
        }
        Some((
            root,
            Self {
                frames: intermediates,
                len: 0,
            },
        ))
    }
}

impl PrivatePageTableAllocator for ReusedPageTableAllocator {
    fn finish(self, root: PhysFrame<Size4KiB>) -> Option<[u64; AGENT_PAGE_TABLE_FRAME_COUNT]> {
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

// SAFETY: each frame comes from one committed whole-address-space allocation
// and is transferred once in the identity order P3, P2, P1.
unsafe impl FrameAllocator<Size4KiB> for ReusedPageTableAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = *self.frames.get(self.len)?.as_ref()?;
        self.len += 1;
        Some(frame)
    }
}
