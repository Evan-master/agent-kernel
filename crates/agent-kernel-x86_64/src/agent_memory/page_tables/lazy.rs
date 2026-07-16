//! One-way activation of the retained Agent lazy-data leaf.
//!
//! This bare-metal page-table child walks the already allocated private Agent
//! tables through the supervisor physical window and writes exactly one absent
//! 4 KiB leaf. It performs no allocation and accepts no arbitrary address or
//! flags; the parent memory owner validates kernel CR3 and frame identity.

use x86_64::{
    structures::paging::{
        page_table::PageTableEntry, OffsetPageTable, PageTable, PageTableFlags, PhysFrame,
    },
    PhysAddr, VirtAddr,
};

use agent_kernel_x86_64::{address_space::AddressSpaceRoots, user_memory::UserMemoryLayout};

use super::{table_pointer, validation::lazy_data_mapping_matches};

const LAZY_DATA_FLAGS: PageTableFlags = PageTableFlags::PRESENT
    .union(PageTableFlags::USER_ACCESSIBLE)
    .union(PageTableFlags::WRITABLE)
    .union(PageTableFlags::NO_EXECUTE);

pub(super) fn activate(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> Option<()> {
    let root = PhysFrame::from_start_address(PhysAddr::new(roots.agent_root())).ok()?;
    let root_pointer = table_pointer(physical_offset, root)?;
    let address = VirtAddr::new(layout.lazy_data_start());
    let leaf_pointer = unsafe { leaf_table_pointer(physical_offset, root_pointer, address)? };
    let leaf_index = usize::from(address.p1_index());

    // SAFETY: the parent owns the inactive Agent root and retained frame. The
    // target entry is required to be unused before this one-way transition.
    unsafe {
        let entry = &mut (&mut *leaf_pointer)[leaf_index];
        if !entry.is_unused() {
            return None;
        }
        entry.set_frame(frame, LAZY_DATA_FLAGS);
    }

    // SAFETY: the root remains exclusively owned and inactive while this
    // scoped mapper validates the newly written leaf.
    let mapper =
        unsafe { OffsetPageTable::new(&mut *root_pointer, VirtAddr::new(physical_offset)) };
    if lazy_data_mapping_matches(&mapper, layout, frame) {
        Some(())
    } else {
        // SAFETY: no ownership escaped and the same exclusive leaf can be
        // returned to its original absent state on validation failure.
        unsafe {
            (&mut *leaf_pointer)[leaf_index].set_unused();
        }
        None
    }
}

unsafe fn leaf_table_pointer(
    physical_offset: u64,
    root_pointer: *mut PageTable,
    address: VirtAddr,
) -> Option<*mut PageTable> {
    let p4 = unsafe { &*root_pointer };
    let p3_frame = next_table_frame(&p4[usize::from(address.p4_index())])?;
    let p3_pointer = table_pointer(physical_offset, p3_frame)?;
    let p3 = unsafe { &*p3_pointer };
    let p2_frame = next_table_frame(&p3[usize::from(address.p3_index())])?;
    let p2_pointer = table_pointer(physical_offset, p2_frame)?;
    let p2 = unsafe { &*p2_pointer };
    let p1_frame = next_table_frame(&p2[usize::from(address.p2_index())])?;
    table_pointer(physical_offset, p1_frame)
}

fn next_table_frame(entry: &PageTableEntry) -> Option<PhysFrame> {
    (!entry.flags().contains(PageTableFlags::HUGE_PAGE)).then(|| entry.frame().ok())?
}
