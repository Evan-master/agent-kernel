//! Agent P4 construction and isolation validation.
//!
//! This child module clones supervisor-only kernel P4 entries into one fresh
//! root, maps the dedicated Agent slot, and validates both roots before any CR3
//! transition. Shared lower tables are never modified through the Agent slot.

mod lazy;
mod validation;

use x86_64::{
    registers::control::Cr3,
    structures::paging::{Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame},
    VirtAddr,
};

use agent_kernel_x86_64::{
    address_space::{AddressSpaceKind, AddressSpaceRoots, AGENT_P4_INDEX},
    user_memory::{UserMemoryLayout, PAGE_BYTES, STACK_PAGE_COUNT},
};

use self::validation::{
    agent_mappings_match, inherited_entries_match, kernel_excludes_agent_region,
    kernel_root_can_be_inherited,
};
use super::{physical_pointer, BootFrameAllocator};

pub(super) fn install(
    physical_offset: u64,
    allocator: &mut BootFrameAllocator<'_>,
    layout: UserMemoryLayout,
    code_frame: PhysFrame,
    signal_frame: PhysFrame,
    stack_frames: &[PhysFrame; STACK_PAGE_COUNT],
    lazy_data_frame: PhysFrame,
) -> Option<AddressSpaceRoots> {
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
    {
        // SAFETY: the fresh root is reachable through the fixed supervisor
        // physical window, and mapped frames are exclusive to this Agent slot.
        let mut agent_mapper =
            unsafe { OffsetPageTable::new(&mut *agent_pointer, VirtAddr::new(physical_offset)) };
        map_page(
            &mut agent_mapper,
            allocator,
            layout.code_start(),
            code_frame,
            code_flags,
        )?;
        map_page(
            &mut agent_mapper,
            allocator,
            layout.signal_start(),
            signal_frame,
            signal_flags,
        )?;
        for (index, frame) in stack_frames.iter().copied().enumerate() {
            map_page(
                &mut agent_mapper,
                allocator,
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
    }

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
    Some(roots)
}

pub(super) fn activate_lazy_data(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> Option<()> {
    lazy::activate(physical_offset, roots, layout, frame)
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
    allocator: &mut BootFrameAllocator<'_>,
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
