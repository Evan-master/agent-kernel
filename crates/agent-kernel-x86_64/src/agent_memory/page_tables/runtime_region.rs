//! Atomic multi-leaf mapping for one private Agent runtime region.
//!
//! This bare-metal page-table child edits a bounded contiguous P1 range while
//! the kernel CR3 is active. It prevalidates all leaves, applies user RW and NX
//! policy, and restores captured entries if bulk deactivation validation fails.

use x86_64::{
    structures::paging::{
        page_table::PageTableEntry, OffsetPageTable, PageTable, PageTableFlags, PhysFrame,
    },
    PhysAddr, VirtAddr,
};

use agent_kernel_x86_64::{
    address_space::AddressSpaceRoots, runtime_frame_pool::MAX_RUNTIME_REGION_PAGES,
    runtime_region::RUNTIME_REGION_SLOT_COUNT, user_memory::UserMemoryLayout,
};

use super::{
    table_pointer,
    validation::{runtime_region_mapping_matches, runtime_region_unmapped},
};

const FLAGS: PageTableFlags = PageTableFlags::PRESENT
    .union(PageTableFlags::USER_ACCESSIBLE)
    .union(PageTableFlags::WRITABLE)
    .union(PageTableFlags::NO_EXECUTE);

pub(super) fn activate(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    start_slot: usize,
    frames: &[PhysFrame],
) -> Option<()> {
    let root = root_pointer(physical_offset, roots)?;
    let entries = entry_pointers(physical_offset, root, layout, start_slot, frames.len())?;
    for entry in entries.iter().copied().take(frames.len()) {
        if !unsafe { &*entry }.is_unused() {
            return None;
        }
    }
    for (entry, frame) in entries.iter().copied().zip(frames.iter().copied()) {
        // SAFETY: every target leaf was prevalidated unused in this inactive
        // private root and every frame belongs to one pool reservation.
        unsafe { (&mut *entry).set_frame(frame, FLAGS) };
    }
    if mappings_match(physical_offset, root, layout, start_slot, frames) {
        Some(())
    } else {
        for entry in entries.iter().copied().take(frames.len()) {
            // SAFETY: these leaves were installed by this uncommitted operation.
            unsafe { (&mut *entry).set_unused() };
        }
        None
    }
}

pub(super) fn is_active(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    start_slot: usize,
    frames: &[PhysFrame],
) -> bool {
    root_pointer(physical_offset, roots)
        .is_some_and(|root| mappings_match(physical_offset, root, layout, start_slot, frames))
}

pub(super) fn is_absent(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    start_slot: usize,
    page_count: usize,
) -> bool {
    root_pointer(physical_offset, roots).is_some_and(|root| {
        absence_range_valid(start_slot, page_count)
            && (0..page_count).all(|page| {
                let Some(address) = layout.runtime_region_page_start(start_slot + page) else {
                    return false;
                };
                // SAFETY: the private root remains live and this mapper only reads.
                let mapper =
                    unsafe { OffsetPageTable::new(&mut *root, VirtAddr::new(physical_offset)) };
                runtime_region_unmapped(&mapper, address)
            })
    })
}

pub(super) fn deactivate(
    physical_offset: u64,
    roots: AddressSpaceRoots,
    layout: UserMemoryLayout,
    start_slot: usize,
    frames: &[PhysFrame],
) -> Option<()> {
    let root = root_pointer(physical_offset, roots)?;
    if !mappings_match(physical_offset, root, layout, start_slot, frames) {
        return None;
    }
    let entries = entry_pointers(physical_offset, root, layout, start_slot, frames.len())?;
    let mut previous: [PageTableEntry; MAX_RUNTIME_REGION_PAGES] =
        core::array::from_fn(|_| PageTableEntry::new());
    for (page, entry) in entries.iter().copied().take(frames.len()).enumerate() {
        // SAFETY: validation bound each pointer to a distinct live P1 entry.
        previous[page] = unsafe { (&*entry).clone() };
        unsafe { (&mut *entry).set_unused() };
    }
    if is_absent(physical_offset, roots, layout, start_slot, frames.len()) {
        Some(())
    } else {
        for (page, entry) in entries.iter().copied().take(frames.len()).enumerate() {
            // SAFETY: restore each captured entry into its original private slot.
            unsafe { *entry = previous[page].clone() };
        }
        None
    }
}

fn mappings_match(
    physical_offset: u64,
    root: *mut PageTable,
    layout: UserMemoryLayout,
    start_slot: usize,
    frames: &[PhysFrame],
) -> bool {
    if !range_valid(start_slot, frames.len()) {
        return false;
    }
    // SAFETY: the private root remains live and this scoped mapper only reads.
    let mapper = unsafe { OffsetPageTable::new(&mut *root, VirtAddr::new(physical_offset)) };
    frames.iter().copied().enumerate().all(|(page, frame)| {
        layout
            .runtime_region_page_start(start_slot + page)
            .is_some_and(|address| runtime_region_mapping_matches(&mapper, address, frame))
    })
}

fn entry_pointers(
    physical_offset: u64,
    root: *mut PageTable,
    layout: UserMemoryLayout,
    start_slot: usize,
    page_count: usize,
) -> Option<[*mut PageTableEntry; MAX_RUNTIME_REGION_PAGES]> {
    if !range_valid(start_slot, page_count) {
        return None;
    }
    let mut entries = [core::ptr::null_mut(); MAX_RUNTIME_REGION_PAGES];
    for (page, entry) in entries.iter_mut().enumerate().take(page_count) {
        let address = VirtAddr::new(layout.runtime_region_page_start(start_slot + page)?);
        let leaf = unsafe { leaf_table_pointer(physical_offset, root, address)? };
        // SAFETY: the private root remains live and each page resolves to one
        // distinct leaf in the prevalidated contiguous range.
        let leaf = unsafe { &mut *leaf };
        *entry = (&mut leaf[address.p1_index()]) as *mut PageTableEntry;
    }
    Some(entries)
}

fn range_valid(start_slot: usize, page_count: usize) -> bool {
    page_count != 0
        && page_count <= MAX_RUNTIME_REGION_PAGES
        && start_slot
            .checked_add(page_count)
            .is_some_and(|end| end <= RUNTIME_REGION_SLOT_COUNT)
}

fn absence_range_valid(start_slot: usize, page_count: usize) -> bool {
    page_count != 0
        && start_slot
            .checked_add(page_count)
            .is_some_and(|end| end <= RUNTIME_REGION_SLOT_COUNT)
}

fn root_pointer(physical_offset: u64, roots: AddressSpaceRoots) -> Option<*mut PageTable> {
    let frame = PhysFrame::from_start_address(PhysAddr::new(roots.agent_root())).ok()?;
    table_pointer(physical_offset, frame)
}

unsafe fn leaf_table_pointer(
    physical_offset: u64,
    root: *mut PageTable,
    address: VirtAddr,
) -> Option<*mut PageTable> {
    let p4 = unsafe { &*root };
    let p3 = table_pointer(physical_offset, next_frame(&p4[address.p4_index()])?)?;
    let p3 = unsafe { &*p3 };
    let p2 = table_pointer(physical_offset, next_frame(&p3[address.p3_index()])?)?;
    let p2 = unsafe { &*p2 };
    table_pointer(physical_offset, next_frame(&p2[address.p2_index()])?)
}

fn next_frame(entry: &PageTableEntry) -> Option<PhysFrame> {
    (!entry.flags().contains(PageTableFlags::HUGE_PAGE)).then(|| entry.frame().ok())?
}
