//! Physical preparation and byte ownership for the global runtime frame pool.
//!
//! This bare-metal Agent-memory child removes fixed frames from BootInfo,
//! exposes validated frame sets to page-table transactions, reads supervisor
//! aliases for inspection, and clears bytes before ledger release.

use bootloader_api::BootInfo;
use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_core::{AgentId, MemoryCellId, ResourceId};
use agent_kernel_x86_64::runtime_frame_pool::{
    RuntimeFrameBinding, RuntimeFramePoolLedger, RuntimeFrameRelease, RuntimeFrameReservation,
    MAX_RUNTIME_REGION_PAGES, RUNTIME_FRAME_POOL_CAPACITY,
};

use super::{
    clear_page, frame_allocator::BootFrameAllocator, page_is_zero, physical_pointer,
    PreparedAgentMemory, PHYSICAL_MEMORY_OFFSET,
};

#[derive(Copy, Clone)]
pub(crate) struct RuntimePhysicalFrameSet {
    frames: [PhysFrame; MAX_RUNTIME_REGION_PAGES],
    page_count: usize,
}

pub(crate) struct RuntimeMemoryPool {
    ledger: RuntimeFramePoolLedger,
    frames: [PhysFrame; RUNTIME_FRAME_POOL_CAPACITY],
    pointers: [*mut u8; RUNTIME_FRAME_POOL_CAPACITY],
}

impl RuntimeMemoryPool {
    pub(crate) fn prepare(boot_info: &mut BootInfo) -> Option<Self> {
        let physical_offset = boot_info.physical_memory_offset.into_option()?;
        if physical_offset != PHYSICAL_MEMORY_OFFSET {
            return None;
        }
        let zero_frame = PhysFrame::from_start_address(PhysAddr::new(0)).ok()?;
        let mut frames = [zero_frame; RUNTIME_FRAME_POOL_CAPACITY];
        let mut pointers = [core::ptr::null_mut(); RUNTIME_FRAME_POOL_CAPACITY];
        let mut allocator = BootFrameAllocator::new(&mut boot_info.memory_regions);
        for index in 0..RUNTIME_FRAME_POOL_CAPACITY {
            let frame = allocator.allocate()?;
            let pointer = physical_pointer(physical_offset, frame)?;
            if frames[..index].contains(&frame) || !clear_page(pointer) {
                return None;
            }
            frames[index] = frame;
            pointers[index] = pointer;
        }
        let pool = Self {
            ledger: RuntimeFramePoolLedger::new(),
            frames,
            pointers,
        };
        pool.all_available_and_zero().then_some(pool)
    }

    pub(crate) fn reserve(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        page_count: usize,
    ) -> Option<RuntimeFrameReservation> {
        let reservation = self.ledger.reserve(agent, resource, page_count)?;
        if self.frame_set_for_reservation(reservation)?.is_zero(self) {
            Some(reservation)
        } else {
            self.ledger.cancel(reservation);
            None
        }
    }

    pub(crate) fn cancel(&mut self, reservation: RuntimeFrameReservation) -> bool {
        self.frame_set_for_reservation(reservation)
            .is_some_and(|frames| frames.is_zero(self))
            && self.ledger.cancel(reservation)
    }

    pub(crate) fn commit(
        &mut self,
        reservation: RuntimeFrameReservation,
        cell: MemoryCellId,
        generation: u64,
    ) -> bool {
        self.ledger.commit_mapping(reservation, cell, generation)
    }

    pub(crate) fn binding(
        &self,
        agent: AgentId,
        resource: ResourceId,
        cell: MemoryCellId,
        generation: u64,
    ) -> Option<RuntimeFrameBinding> {
        self.ledger.binding(agent, resource, cell, generation)
    }

    pub(crate) fn prepare_release(
        &self,
        agent: AgentId,
        resource: ResourceId,
        cell: MemoryCellId,
        generation: u64,
    ) -> Option<RuntimeFrameRelease> {
        self.ledger
            .prepare_release(agent, resource, cell, generation)
    }

    pub(crate) fn release(&mut self, release: RuntimeFrameRelease) -> bool {
        if self.ledger.prepare_release(
            release.agent(),
            release.resource(),
            release.cell(),
            release.generation(),
        ) != Some(release)
        {
            return false;
        }
        let Some(frames) = self.frame_set_for_release(release) else {
            return false;
        };
        if !frames.clear(self) || !self.ledger.commit_release(release) {
            return false;
        }
        frames.is_zero(self)
    }

    pub(crate) fn observe(&self, binding: RuntimeFrameBinding) -> Option<(u64, u64)> {
        if self.ledger.binding(
            binding.agent(),
            binding.resource(),
            binding.cell(),
            binding.generation(),
        ) != Some(binding)
        {
            return None;
        }
        let frames = self.frame_set_for_binding(binding)?;
        let first = frames.pointer(self, 0)?;
        let last = frames.pointer(self, frames.page_count().checked_sub(1)?)?;
        // SAFETY: the ledger and frame set bind both pointers to live exclusive
        // pool frames while the kernel reads through supervisor aliases.
        Some(unsafe {
            (
                first.cast::<u64>().read_volatile(),
                last.cast::<u64>().read_volatile(),
            )
        })
    }

    pub(crate) fn frame_set_for_reservation(
        &self,
        reservation: RuntimeFrameReservation,
    ) -> Option<RuntimePhysicalFrameSet> {
        self.frame_set(reservation.page_count(), |page| {
            reservation.frame_index(page)
        })
    }

    pub(crate) fn frame_set_for_binding(
        &self,
        binding: RuntimeFrameBinding,
    ) -> Option<RuntimePhysicalFrameSet> {
        self.frame_set(binding.page_count(), |page| binding.frame_index(page))
    }

    pub(crate) fn frame_set_for_release(
        &self,
        release: RuntimeFrameRelease,
    ) -> Option<RuntimePhysicalFrameSet> {
        self.frame_set(release.page_count(), |page| release.frame_index(page))
    }

    pub(crate) fn agent_is_clear(&self, agent: AgentId) -> bool {
        self.ledger.agent_is_clear(agent)
    }

    pub(crate) fn all_available_and_zero(&self) -> bool {
        self.ledger.all_available() && self.pointers.iter().copied().all(page_is_zero)
    }

    pub(crate) fn is_disjoint_from(&self, memory: &PreparedAgentMemory) -> bool {
        let content = memory.identity.content_frames();
        self.frames.iter().all(|frame| {
            let address = frame.start_address().as_u64();
            address != memory.identity.root() && !content.contains(&address)
        })
    }

    fn frame_set(
        &self,
        page_count: usize,
        index_at: impl Fn(usize) -> Option<usize>,
    ) -> Option<RuntimePhysicalFrameSet> {
        if page_count == 0 || page_count > MAX_RUNTIME_REGION_PAGES {
            return None;
        }
        let zero_frame = PhysFrame::from_start_address(PhysAddr::new(0)).ok()?;
        let mut frames = [zero_frame; MAX_RUNTIME_REGION_PAGES];
        for (page, frame) in frames.iter_mut().enumerate().take(page_count) {
            *frame = *self.frames.get(index_at(page)?)?;
        }
        Some(RuntimePhysicalFrameSet { frames, page_count })
    }
}

mod frame_set;
