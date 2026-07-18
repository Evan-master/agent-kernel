//! Physical ownership and reversible mapping for one Agent runtime page.
//!
//! This bare-metal memory child coordinates the pure ledger with one retained
//! frame and one fixed page-table leaf. It emits no semantic events; the native
//! call executor surrounds these effects with public MemoryCell and Resource
//! facade commits.

use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_core::{MemoryCellId, MemoryValue, ResourceId};
use agent_kernel_x86_64::{
    runtime_page::{RuntimePageRelease, RuntimePageReservation, RUNTIME_PAGE_ACCESS_READ_WRITE},
    user_memory::{PAGE_BYTES, STACK_PAGE_COUNT},
};

use super::{
    clear_page, page_tables, physical_pointer, PreparedAgentMemory, PHYSICAL_MEMORY_OFFSET,
};

impl PreparedAgentMemory {
    pub(crate) fn prepare_runtime_page_allocation(
        &mut self,
        resource: ResourceId,
    ) -> Option<(RuntimePageReservation, MemoryValue)> {
        if !self.kernel_address_space_active()
            || !self.runtime_page.is_available()
            || !self.runtime_page_is_absent()
            || !page_is_zero(self.runtime_page_pointer)
        {
            return None;
        }
        let reservation = self.runtime_page.reserve(resource)?;
        let frame = self.runtime_page_frame()?;
        if page_tables::activate_runtime_page(
            PHYSICAL_MEMORY_OFFSET,
            self.roots,
            self.layout,
            frame,
        )
        .is_none()
        {
            self.runtime_page.cancel(reservation);
            return None;
        }
        Some((
            reservation,
            MemoryValue::new([
                self.layout.runtime_page_start(),
                PAGE_BYTES,
                RUNTIME_PAGE_ACCESS_READ_WRITE,
                reservation.generation(),
            ]),
        ))
    }

    pub(crate) fn commit_runtime_page_allocation(
        &mut self,
        reservation: RuntimePageReservation,
        cell: MemoryCellId,
    ) -> bool {
        self.runtime_page.commit_mapping(reservation, cell)
    }

    pub(crate) fn rollback_runtime_page_allocation(
        &mut self,
        reservation: RuntimePageReservation,
    ) -> bool {
        let Some(frame) = self.runtime_page_frame() else {
            return false;
        };
        page_tables::deactivate_runtime_page(PHYSICAL_MEMORY_OFFSET, self.roots, self.layout, frame)
            .is_some()
            && clear_page(self.runtime_page_pointer)
            && self.runtime_page.cancel(reservation)
            && self.runtime_page_is_absent()
            && page_is_zero(self.runtime_page_pointer)
    }

    pub(crate) fn inspect_runtime_page(
        &mut self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
    ) -> Option<(u64, u64)> {
        let generation = self.validate_runtime_page(resource, cell, descriptor)?;
        // SAFETY: the binding and page table identify this exclusive retained
        // frame, and the kernel reads through its supervisor physical alias.
        let value = unsafe { self.runtime_page_pointer.cast::<u64>().read_volatile() };
        self.runtime_page_observation = Some(value);
        Some((value, generation))
    }

    pub(crate) fn prepare_runtime_page_release(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
    ) -> Option<RuntimePageRelease> {
        self.validate_runtime_page(resource, cell, descriptor)?;
        self.runtime_page.prepare_release(resource, cell)
    }

    pub(crate) fn release_runtime_page(&mut self, release: RuntimePageRelease) -> bool {
        let Some(frame) = self.runtime_page_frame() else {
            return false;
        };
        if !self.runtime_page_is_active()
            || page_tables::deactivate_runtime_page(
                PHYSICAL_MEMORY_OFFSET,
                self.roots,
                self.layout,
                frame,
            )
            .is_none()
            || !clear_page(self.runtime_page_pointer)
            || !self.runtime_page.commit_release(release)
        {
            return false;
        }
        self.runtime_page_released(release.generation())
    }

    pub(crate) fn runtime_page_generation(&self) -> u64 {
        self.runtime_page.generation()
    }

    pub(crate) fn runtime_page_observation(&self) -> Option<u64> {
        self.runtime_page_observation
    }

    pub(crate) fn runtime_page_released(&self, generation: u64) -> bool {
        generation != 0
            && self.runtime_page.generation() == generation
            && self.runtime_page.is_available()
            && self.runtime_page_is_absent()
            && page_is_zero(self.runtime_page_pointer)
    }

    fn runtime_page_frame(&self) -> Option<PhysFrame> {
        let address = self.identity.content_frames()[STACK_PAGE_COUNT + 3];
        let frame = PhysFrame::from_start_address(PhysAddr::new(address)).ok()?;
        (physical_pointer(PHYSICAL_MEMORY_OFFSET, frame)? == self.runtime_page_pointer)
            .then_some(frame)
    }

    fn validate_runtime_page(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
        descriptor: MemoryValue,
    ) -> Option<u64> {
        let binding = self.runtime_page.binding()?;
        (binding.resource() == resource
            && binding.cell() == cell
            && descriptor
                == MemoryValue::new([
                    self.layout.runtime_page_start(),
                    PAGE_BYTES,
                    RUNTIME_PAGE_ACCESS_READ_WRITE,
                    binding.generation(),
                ])
            && self.runtime_page_is_active())
        .then_some(binding.generation())
    }

    fn runtime_page_is_active(&self) -> bool {
        self.runtime_page_frame().is_some_and(|frame| {
            page_tables::runtime_page_is_active(
                PHYSICAL_MEMORY_OFFSET,
                self.roots,
                self.layout,
                frame,
            )
        })
    }

    fn runtime_page_is_absent(&self) -> bool {
        page_tables::runtime_page_is_absent(PHYSICAL_MEMORY_OFFSET, self.roots, self.layout)
    }
}

fn page_is_zero(pointer: *mut u8) -> bool {
    let mut offset = 0;
    while offset < PAGE_BYTES as usize {
        // SAFETY: the pointer names the retained exclusive runtime frame.
        if unsafe { pointer.add(offset).read_volatile() } != 0 {
            return false;
        }
        offset += 1;
    }
    true
}
