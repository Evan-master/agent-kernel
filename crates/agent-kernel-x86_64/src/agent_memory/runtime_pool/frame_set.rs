//! Validated physical-frame slices and byte access for one pool transaction.
//!
//! This bare-metal Agent-memory child resolves retained supervisor aliases,
//! checks zero state, and clears complete bounded frame sets before reuse.

use x86_64::structures::paging::PhysFrame;

use super::{RuntimeMemoryPool, RuntimePhysicalFrameSet};
use crate::agent_memory::{clear_page, page_is_zero};

impl RuntimePhysicalFrameSet {
    pub(crate) const fn page_count(self) -> usize {
        self.page_count
    }

    pub(crate) fn as_slice(&self) -> &[PhysFrame] {
        &self.frames[..self.page_count]
    }

    pub(super) fn pointer(self, pool: &RuntimeMemoryPool, page: usize) -> Option<*mut u8> {
        let frame = *self.frames.get(page)?;
        let index = pool
            .frames
            .iter()
            .position(|candidate| *candidate == frame)?;
        pool.pointers.get(index).copied()
    }

    pub(super) fn is_zero(self, pool: &RuntimeMemoryPool) -> bool {
        (0..self.page_count).all(|page| self.pointer(pool, page).is_some_and(page_is_zero))
    }

    pub(super) fn clear(self, pool: &RuntimeMemoryPool) -> bool {
        (0..self.page_count).all(|page| self.pointer(pool, page).is_some_and(clear_page))
    }
}
