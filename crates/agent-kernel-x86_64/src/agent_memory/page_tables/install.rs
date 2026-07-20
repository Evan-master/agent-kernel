//! Private Agent root construction for boot and reclaimed frame ownership.

use x86_64::{
    registers::control::Cr3,
    structures::paging::{Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame},
    VirtAddr,
};

use agent_kernel_x86_64::{
    address_space::{AddressSpaceRoots, AGENT_P4_INDEX, AGENT_PAGE_TABLE_FRAME_COUNT},
    user_memory::{UserMemoryLayout, PAGE_BYTES, STACK_PAGE_COUNT},
};

use super::ownership::{
    PrivatePageTableAllocator, ReusedPageTableAllocator, TrackedPageTableAllocator,
};
use super::validation::{
    agent_mappings_match, inherited_entries_match, kernel_excludes_agent_region,
    kernel_root_can_be_inherited,
};
use super::{kernel_is_active, table_pointer, InstalledAgentPageTables};
use crate::agent_memory::BootFrameAllocator;

pub(in crate::agent_memory) fn install(
    physical_offset: u64,
    allocator: &mut BootFrameAllocator<'_>,
    layout: UserMemoryLayout,
    code_frame: PhysFrame,
    signal_frame: PhysFrame,
    stack_frames: &[PhysFrame; STACK_PAGE_COUNT],
    lazy_data_frame: PhysFrame,
    call_data_frame: PhysFrame,
) -> Option<InstalledAgentPageTables> {
    let agent_frame = allocator.allocate()?;
    let table_allocator = TrackedPageTableAllocator::new(allocator);
    install_with_allocator(
        physical_offset,
        layout,
        code_frame,
        signal_frame,
        stack_frames,
        lazy_data_frame,
        call_data_frame,
        agent_frame,
        table_allocator,
    )
}

pub(in crate::agent_memory) fn install_reused(
    physical_offset: u64,
    private_frames: [u64; AGENT_PAGE_TABLE_FRAME_COUNT],
    layout: UserMemoryLayout,
    code_frame: PhysFrame,
    signal_frame: PhysFrame,
    stack_frames: &[PhysFrame; STACK_PAGE_COUNT],
    lazy_data_frame: PhysFrame,
    call_data_frame: PhysFrame,
) -> Option<InstalledAgentPageTables> {
    let (agent_frame, table_allocator) = ReusedPageTableAllocator::new(private_frames)?;
    install_with_allocator(
        physical_offset,
        layout,
        code_frame,
        signal_frame,
        stack_frames,
        lazy_data_frame,
        call_data_frame,
        agent_frame,
        table_allocator,
    )
}

fn install_with_allocator(
    physical_offset: u64,
    layout: UserMemoryLayout,
    code_frame: PhysFrame,
    signal_frame: PhysFrame,
    stack_frames: &[PhysFrame; STACK_PAGE_COUNT],
    lazy_data_frame: PhysFrame,
    call_data_frame: PhysFrame,
    agent_frame: PhysFrame,
    mut table_allocator: impl PrivatePageTableAllocator,
) -> Option<InstalledAgentPageTables> {
    let (kernel_frame, control) = Cr3::read_raw();
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
        map_page(
            &mut agent_mapper,
            &mut table_allocator,
            layout.code_start(),
            code_frame,
            code_flags,
        )?;
        map_page(
            &mut agent_mapper,
            &mut table_allocator,
            layout.signal_start(),
            signal_frame,
            signal_flags,
        )?;
        for (index, frame) in stack_frames.iter().copied().enumerate() {
            map_page(
                &mut agent_mapper,
                &mut table_allocator,
                layout.stack_bottom() + PAGE_BYTES * index as u64,
                frame,
                stack_flags,
            )?;
        }
        map_page(
            &mut agent_mapper,
            &mut table_allocator,
            layout.call_data_start(),
            call_data_frame,
            stack_flags,
        )?;
        if !agent_mappings_match(
            &agent_mapper,
            layout,
            code_frame,
            signal_frame,
            stack_frames,
            lazy_data_frame,
            call_data_frame,
        ) {
            return None;
        }
        table_allocator.finish(agent_frame)?
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
