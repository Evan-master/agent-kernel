//! Reversible leaf mapping for one retained Agent runtime page.
//!
//! This bare-metal page-table child edits one fixed private P1 entry while the
//! kernel CR3 is active. It accepts an exclusive retained frame, applies user
//! read/write and NX policy, validates every transition, and restores the leaf
//! if deactivation validation reports an unexpected result.

use x86_64::{
    structures::paging::{
        page_table::PageTableEntry, OffsetPageTable, PageTable, PageTableFlags, PhysFrame,
    },
    PhysAddr, VirtAddr,
};

use agent_kernel_x86_64::{address_space::AddressSpaceRoots, user_memory::UserMemoryLayout};

use super::{
    table_pointer,
    validation::{runtime_page_mapping_matches, runtime_page_unmapped},
};

const FLAGS: PageTableFlags = PageTableFlags::PRESENT
    .union(PageTableFlags::USER_ACCESSIBLE)
    .union(PageTableFlags::WRITABLE)
    .union(PageTableFlags::NO_EXECUTE);

pub(super) fn activate(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> Option<()> {
    let root_pointer = root_pointer(physical_offset, roots)?;
    let address = VirtAddr::new(layout.runtime_page_start());
    let leaf_pointer = unsafe { leaf_table_pointer(physical_offset, root_pointer, address)? };
    let leaf_index = usize::from(address.p1_index());

    // SAFETY: the private root is inactive and the fixed leaf is required to
    // be unused. The retained frame belongs exclusively to this Agent memory.
    unsafe {
        let entry = &mut (&mut *leaf_pointer)[leaf_index];
        if !entry.is_unused() {
            return None;
        }
        entry.set_frame(frame, FLAGS);
    }
    if mapping_matches(physical_offset, root_pointer, layout, frame) {
        Some(())
    } else {
        // SAFETY: the same exclusive leaf has not escaped this transition.
        unsafe {
            (&mut *leaf_pointer)[leaf_index].set_unused();
        }
        None
    }
}

pub(super) fn is_active(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> bool {
    root_pointer(physical_offset, roots)
        .is_some_and(|root| mapping_matches(physical_offset, root, layout, frame))
}

pub(super) fn is_absent(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
) -> bool {
    root_pointer(physical_offset, roots).is_some_and(|root| {
        // SAFETY: the private root remains live and this mapper only reads.
        let mapper = unsafe { OffsetPageTable::new(&mut *root, VirtAddr::new(physical_offset)) };
        runtime_page_unmapped(&mapper, layout)
    })
}

pub(super) fn deactivate(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> Option<()> {
    let root_pointer = root_pointer(physical_offset, roots)?;
    if !mapping_matches(physical_offset, root_pointer, layout, frame) {
        return None;
    }
    let address = VirtAddr::new(layout.runtime_page_start());
    let leaf_pointer = unsafe { leaf_table_pointer(physical_offset, root_pointer, address)? };
    let leaf_index = usize::from(address.p1_index());
    let previous = unsafe { (&*leaf_pointer)[leaf_index].clone() };

    // SAFETY: validation bound this exact private leaf to `frame` and no Agent
    // CR3 is active while the kernel handles the call.
    unsafe {
        (&mut *leaf_pointer)[leaf_index].set_unused();
    }
    if is_absent(physical_offset, roots, layout) {
        Some(())
    } else {
        // SAFETY: restore the captured entry into the same exclusive P1 slot.
        unsafe {
            (&mut *leaf_pointer)[leaf_index] = previous;
        }
        None
    }
}

fn root_pointer(physical_offset: u64, roots: AddressSpaceRoots) -> Option<*mut PageTable> {
    let frame = PhysFrame::from_start_address(PhysAddr::new(roots.agent_root())).ok()?;
    table_pointer(physical_offset, frame)
}

fn mapping_matches(
    physical_offset: u64,
    root_pointer: *mut PageTable,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> bool {
    // SAFETY: the private root remains live and this scoped mapper only reads.
    let mapper =
        unsafe { OffsetPageTable::new(&mut *root_pointer, VirtAddr::new(physical_offset)) };
    runtime_page_mapping_matches(&mapper, layout, frame)
}

unsafe fn leaf_table_pointer(
    physical_offset: u64,
    root_pointer: *mut PageTable,
    address: VirtAddr,
) -> Option<*mut PageTable> {
    let p4 = unsafe { &*root_pointer };
    let p3 = table_pointer(physical_offset, next_table_frame(&p4[address.p4_index()])?)?;
    let p3 = unsafe { &*p3 };
    let p2 = table_pointer(physical_offset, next_table_frame(&p3[address.p3_index()])?)?;
    let p2 = unsafe { &*p2 };
    table_pointer(physical_offset, next_table_frame(&p2[address.p2_index()])?)
}

fn next_table_frame(entry: &PageTableEntry) -> Option<PhysFrame> {
    (!entry.flags().contains(PageTableFlags::HUGE_PAGE)).then(|| entry.frame().ok())?
}
