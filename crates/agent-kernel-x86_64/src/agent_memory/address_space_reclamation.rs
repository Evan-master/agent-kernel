//! Physical teardown for one completed native Agent address space.
//!
//! This bare-metal Agent-memory child preflights complete ownership against a
//! bounded pool, clears private content and page-table frames under the kernel
//! CR3, verifies every byte, and transfers the whole frame set atomically.

use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_x86_64::{
    address_space::{AgentMemoryIdentity, AGENT_OWNED_FRAME_COUNT},
    address_space_reclamation::{AddressSpaceFramePool, AddressSpaceReclamation},
};

use super::{
    clear_page, page_is_zero, physical_pointer, PreparedAgentMemory, PHYSICAL_MEMORY_OFFSET,
};

pub(crate) const NATIVE_ADDRESS_SPACE_CAPACITY: usize = 6;
pub(crate) const NATIVE_ADDRESS_SPACE_FRAME_CAPACITY: usize =
    NATIVE_ADDRESS_SPACE_CAPACITY * AGENT_OWNED_FRAME_COUNT;

#[derive(Copy, Clone)]
pub(crate) struct NativeAddressSpaceFramePool {
    ledger: AddressSpaceFramePool<NATIVE_ADDRESS_SPACE_FRAME_CAPACITY>,
}

#[derive(Copy, Clone)]
pub(crate) struct ReclaimedAgentAddressSpace {
    root: u64,
    frame_count: usize,
}

impl NativeAddressSpaceFramePool {
    pub(crate) const fn new() -> Self {
        Self {
            ledger: AddressSpaceFramePool::new(),
        }
    }

    pub(crate) fn prepare(&self, identity: AgentMemoryIdentity) -> Option<AddressSpaceReclamation> {
        self.ledger.prepare(identity)
    }

    pub(crate) fn preview_commit(&mut self, reclamation: AddressSpaceReclamation) -> bool {
        self.ledger.commit(reclamation)
    }

    fn commit_zeroed(&mut self, reclamation: AddressSpaceReclamation) -> bool {
        reclamation
            .identity()
            .owned_frames()
            .into_iter()
            .all(frame_is_zero)
            && self.ledger.commit(reclamation)
    }

    pub(crate) fn all_reclaimed_and_zero(&self) -> bool {
        self.ledger.len() == NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
            && self.ledger.frames().iter().copied().all(frame_is_zero)
    }

    pub(crate) const fn len(&self) -> usize {
        self.ledger.len()
    }
}

impl PreparedAgentMemory {
    pub(crate) fn prepare_address_space_reclamation(
        &self,
        pool: &NativeAddressSpaceFramePool,
    ) -> Option<AddressSpaceReclamation> {
        if !self.kernel_address_space_active()
            || !self.runtime_memory_is_clear()
            || self.identity.root() != self.roots.agent_root()
        {
            return None;
        }
        pool.prepare(self.identity)
    }

    pub(crate) fn reclaim_address_space(
        self,
        pool: &mut NativeAddressSpaceFramePool,
        reclamation: AddressSpaceReclamation,
    ) -> Option<ReclaimedAgentAddressSpace> {
        if self.prepare_address_space_reclamation(pool)? != reclamation
            || reclamation.identity() != self.identity
        {
            return None;
        }
        for frame in self.identity.content_frames() {
            clear_frame(frame)?;
        }
        let tables = self.identity.page_table_frames();
        for frame in tables[1..].iter().rev().copied() {
            clear_frame(frame)?;
        }
        clear_frame(tables[0])?;
        if !self.identity.owned_frames().into_iter().all(frame_is_zero)
            || !pool.commit_zeroed(reclamation)
        {
            return None;
        }
        Some(ReclaimedAgentAddressSpace {
            root: self.identity.root(),
            frame_count: reclamation.frame_count(),
        })
    }
}

impl ReclaimedAgentAddressSpace {
    pub(crate) const fn root(self) -> u64 {
        self.root
    }

    pub(crate) const fn frame_count(self) -> usize {
        self.frame_count
    }
}

fn clear_frame(address: u64) -> Option<()> {
    let frame = PhysFrame::from_start_address(PhysAddr::new(address)).ok()?;
    clear_page(physical_pointer(PHYSICAL_MEMORY_OFFSET, frame)?).then_some(())
}

fn frame_is_zero(address: u64) -> bool {
    PhysFrame::from_start_address(PhysAddr::new(address))
        .ok()
        .and_then(|frame| physical_pointer(PHYSICAL_MEMORY_OFFSET, frame))
        .is_some_and(page_is_zero)
}
