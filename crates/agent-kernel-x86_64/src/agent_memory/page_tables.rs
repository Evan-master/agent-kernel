//! Agent P4 construction and isolation validation.
//!
//! This child module clones supervisor-only kernel P4 entries into one fresh
//! root, maps the dedicated Agent slot, and validates both roots before any CR3
//! transition. Shared lower tables are never modified through the Agent slot.

mod lazy;
mod ownership;
mod runtime_page;
mod runtime_region;
mod validation;

use x86_64::{
    registers::control::Cr3,
    structures::paging::{Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame},
    VirtAddr,
};

use agent_kernel_x86_64::{
    address_space::{
        AddressSpaceKind, AddressSpaceRoots, AGENT_P4_INDEX, AGENT_PAGE_TABLE_FRAME_COUNT,
    },
    user_memory::{UserMemoryLayout, PAGE_BYTES, STACK_PAGE_COUNT},
};

use self::ownership::TrackedPageTableAllocator;
use self::validation::{
    agent_mappings_match, inherited_entries_match, kernel_excludes_agent_region,
    kernel_root_can_be_inherited,
};
use super::{physical_pointer, BootFrameAllocator};

pub(super) struct InstalledAgentPageTables {
    roots: AddressSpaceRoots,
    private_frames: [u64; AGENT_PAGE_TABLE_FRAME_COUNT],
}

pub(super) fn install(
    physical_offset: u64,
    allocator: &mut BootFrameAllocator<'_>,
    layout: UserMemoryLayout,
    code_frame: PhysFrame,
    signal_frame: PhysFrame,
    stack_frames: &[PhysFrame; STACK_PAGE_COUNT],
    lazy_data_frame: PhysFrame,
) -> Option<InstalledAgentPageTables> {
    let (kernel_frame, control) = Cr3::read_raw();
    let agent_frame = allocator.allocate()?;
    let roots = AddressSpaceRoots::new(
        kernel_frame.start_address().as_u64(),
        agent_frame.start_address().as_u64(),
        u64::from(control),
    )?;
    let kernel_pointer = table_pointer(physical_offset, kernel_frame)?;
    let agent_pointer = table_pointer(physical_offset, agent_frame)?;

    // SAFETY: the active root is read-only here, and the fresh Agent root frame
    // is exclusively owned. Cloned lower tables remain supervisor-only.
    unsafe {
        if !kernel_root_can_be_inherited(&*kernel_pointer) {
            return None;
        }
        agent_pointer.write((&*kernel_pointer).clone());
        (&mut *agent_pointer)[AGENT_P4_INDEX].set_unused();
    }

    let code_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    let signal_flags =
        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::NO_EXECUTE;
    let stack_flags = signal_flags | PageTableFlags::WRITABLE;
    let private_frames = {
        // SAFETY: the fresh root is reachable through the fixed supervisor
        // physical window, and mapped frames are exclusive to this Agent slot.
        let mut agent_mapper =
            unsafe { OffsetPageTable::new(&mut *agent_pointer, VirtAddr::new(physical_offset)) };
        let mut tracked_allocator = TrackedPageTableAllocator::new(allocator);
        map_page(
            &mut agent_mapper,
            &mut tracked_allocator,
            layout.code_start(),
            code_frame,
            code_flags,
        )?;
        map_page(
            &mut agent_mapper,
            &mut tracked_allocator,
            layout.signal_start(),
            signal_frame,
            signal_flags,
        )?;
        for (index, frame) in stack_frames.iter().copied().enumerate() {
            map_page(
                &mut agent_mapper,
                &mut tracked_allocator,
                layout.stack_bottom() + PAGE_BYTES * index as u64,
                frame,
                stack_flags,
            )?;
        }
        if !agent_mappings_match(
            &agent_mapper,
            layout,
            code_frame,
            signal_frame,
            stack_frames,
            lazy_data_frame,
        ) {
            return None;
        }
        tracked_allocator.finish(agent_frame)?
    };

    // SAFETY: both pointers name distinct live P4 frames. Hardware-managed
    // accessed/dirty bits are ignored by the inherited-entry comparison.
    unsafe {
        if !inherited_entries_match(&*kernel_pointer, &*agent_pointer) {
            return None;
        }
    }
    if !kernel_excludes_agent_region(physical_offset, kernel_pointer, layout)
        || !kernel_is_active(roots)
    {
        return None;
    }
    Some(InstalledAgentPageTables {
        roots,
        private_frames,
    })
}

impl InstalledAgentPageTables {
    pub(super) const fn roots(&self) -> AddressSpaceRoots {
        self.roots
    }

    pub(super) const fn private_frames(&self) -> [u64; AGENT_PAGE_TABLE_FRAME_COUNT] {
        self.private_frames
    }
}

pub(super) fn activate_lazy_data(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> Option<()> {
    lazy::activate(physical_offset, roots, layout, frame)
}

pub(super) fn activate_runtime_page(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> Option<()> {
    runtime_page::activate(physical_offset, roots, layout, frame)
}

pub(super) fn runtime_page_is_active(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> bool {
    runtime_page::is_active(physical_offset, roots, layout, frame)
}

pub(super) fn runtime_page_is_absent(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
) -> bool {
    runtime_page::is_absent(physical_offset, roots, layout)
}

pub(super) fn deactivate_runtime_page(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> Option<()> {
    runtime_page::deactivate(physical_offset, roots, layout, frame)
}

pub(super) fn activate_runtime_region(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    start_slot: usize,
    frames: &[PhysFrame],
) -> Option<()> {
    runtime_region::activate(physical_offset, roots, layout, start_slot, frames)
}

pub(super) fn runtime_region_is_active(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    start_slot: usize,
    frames: &[PhysFrame],
) -> bool {
    runtime_region::is_active(physical_offset, roots, layout, start_slot, frames)
}

pub(super) fn runtime_region_is_absent(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    start_slot: usize,
    page_count: usize,
) -> bool {
    runtime_region::is_absent(physical_offset, roots, layout, start_slot, page_count)
}

pub(super) fn deactivate_runtime_region(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    start_slot: usize,
    frames: &[PhysFrame],
) -> Option<()> {
    runtime_region::deactivate(physical_offset, roots, layout, start_slot, frames)
}

pub(super) fn kernel_is_active(roots: AddressSpaceRoots) -> bool {
    roots.classify(current_raw_cr3()) == Some(AddressSpaceKind::Kernel)
}

fn current_raw_cr3() -> u64 {
    let (frame, control) = Cr3::read_raw();
    frame.start_address().as_u64() | u64::from(control)
}

fn table_pointer(physical_offset: u64, frame: PhysFrame) -> Option<*mut PageTable> {
    Some(physical_pointer(physical_offset, frame)?.cast())
}

fn map_page(
    mapper: &mut OffsetPageTable<'_>,
    allocator: &mut impl x86_64::structures::paging::FrameAllocator<
        x86_64::structures::paging::Size4KiB,
    >,
    virtual_address: u64,
    frame: PhysFrame,
    flags: PageTableFlags,
) -> Option<()> {
    let page = Page::from_start_address(VirtAddr::new(virtual_address)).ok()?;
    // SAFETY: the virtual page is unused in the private Agent slot and the
    // content frame is exclusively owned by this address space.
    unsafe {
        mapper.map_to(page, frame, flags, allocator).ok()?.flush();
    }
    Some(())
}
