//! Read-only validation for the kernel and Agent page-table roots.
//!
//! This module proves inherited supervisor mappings, least-authority Agent page
//! flags, and total kernel exclusion of the dedicated Agent virtual region.
//! Hardware-managed accessed and dirty bits are not treated as policy state.

use x86_64::{
    structures::paging::{
        mapper::{MappedFrame, Translate, TranslateResult},
        page_table::PageTableEntry,
        OffsetPageTable, PageTable, PageTableFlags, PhysFrame,
    },
    VirtAddr,
};

use agent_kernel_x86_64::{
    address_space::{AGENT_P4_INDEX, P4_ENTRY_COUNT},
    runtime_region::RUNTIME_REGION_SLOT_COUNT,
    user_memory::{UserMemoryLayout, PAGE_BYTES, STACK_PAGE_COUNT},
};

pub(super) fn kernel_root_can_be_inherited(kernel: &PageTable) -> bool {
    kernel[AGENT_P4_INDEX].is_unused()
        && kernel
            .iter()
            .enumerate()
            .all(|(index, entry)| index == AGENT_P4_INDEX || supervisor_entry(entry))
}

pub(super) fn inherited_entries_match(kernel: &PageTable, agent: &PageTable) -> bool {
    if !kernel[AGENT_P4_INDEX].is_unused()
        || agent[AGENT_P4_INDEX].is_unused()
        || !agent[AGENT_P4_INDEX].flags().contains(
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        )
    {
        return false;
    }
    (0..P4_ENTRY_COUNT).all(|index| {
        index == AGENT_P4_INDEX
            || (kernel[index].addr() == agent[index].addr()
                && stable_flags(&kernel[index]) == stable_flags(&agent[index])
                && supervisor_entry(&agent[index]))
    })
}

pub(super) fn kernel_excludes_agent_region(
    physical_offset: u64,
    kernel_pointer: *mut PageTable,
    layout: UserMemoryLayout,
) -> bool {
    // SAFETY: the active kernel root remains live and this scoped mapper only
    // performs translations; no Agent mapper reference exists concurrently.
    let mapper =
        unsafe { OffsetPageTable::new(&mut *kernel_pointer, VirtAddr::new(physical_offset)) };
    mapper
        .translate_addr(VirtAddr::new(layout.code_start()))
        .is_none()
        && mapper
            .translate_addr(VirtAddr::new(layout.signal_start()))
            .is_none()
        && mapper
            .translate_addr(VirtAddr::new(layout.guard_start()))
            .is_none()
        && mapper
            .translate_addr(VirtAddr::new(layout.lazy_data_start()))
            .is_none()
        && mapper
            .translate_addr(VirtAddr::new(layout.runtime_page_start()))
            .is_none()
        && (0..RUNTIME_REGION_SLOT_COUNT).all(|slot| {
            layout
                .runtime_region_page_start(slot)
                .is_some_and(|address| mapper.translate_addr(VirtAddr::new(address)).is_none())
        })
        && (0..STACK_PAGE_COUNT).all(|index| {
            mapper
                .translate_addr(VirtAddr::new(
                    layout.stack_bottom() + PAGE_BYTES * index as u64,
                ))
                .is_none()
        })
}

pub(super) fn agent_mappings_match(
    mapper: &OffsetPageTable<'_>,
    layout: UserMemoryLayout,
    code_frame: PhysFrame,
    signal_frame: PhysFrame,
    stack_frames: &[PhysFrame; STACK_PAGE_COUNT],
    _lazy_data_frame: PhysFrame,
) -> bool {
    let code_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    let signal_flags =
        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::NO_EXECUTE;
    let stack_flags = signal_flags | PageTableFlags::WRITABLE;
    mapping_matches(
        mapper,
        layout.code_start(),
        code_frame,
        code_flags,
        PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
    ) && mapping_matches(
        mapper,
        layout.signal_start(),
        signal_frame,
        signal_flags,
        PageTableFlags::WRITABLE,
    ) && mapper
        .translate_addr(VirtAddr::new(layout.guard_start()))
        .is_none()
        && stack_frames
            .iter()
            .copied()
            .enumerate()
            .all(|(index, frame)| {
                mapping_matches(
                    mapper,
                    layout.stack_bottom() + PAGE_BYTES * index as u64,
                    frame,
                    stack_flags,
                    PageTableFlags::empty(),
                )
            })
        && mapper
            .translate_addr(VirtAddr::new(layout.lazy_data_start()))
            .is_none()
        && mapper
            .translate_addr(VirtAddr::new(layout.runtime_page_start()))
            .is_none()
        && (0..RUNTIME_REGION_SLOT_COUNT).all(|slot| {
            layout
                .runtime_region_page_start(slot)
                .is_some_and(|address| mapper.translate_addr(VirtAddr::new(address)).is_none())
        })
}

pub(super) fn lazy_data_mapping_matches(
    mapper: &OffsetPageTable<'_>,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> bool {
    let flags = PageTableFlags::PRESENT
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_EXECUTE;
    mapping_matches(
        mapper,
        layout.lazy_data_start(),
        frame,
        flags,
        PageTableFlags::HUGE_PAGE,
    )
}

pub(super) fn runtime_page_mapping_matches(
    mapper: &OffsetPageTable<'_>,
    layout: UserMemoryLayout,
    frame: PhysFrame,
) -> bool {
    let flags = PageTableFlags::PRESENT
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_EXECUTE;
    mapping_matches(
        mapper,
        layout.runtime_page_start(),
        frame,
        flags,
        PageTableFlags::HUGE_PAGE,
    )
}

pub(super) fn runtime_page_unmapped(
    mapper: &OffsetPageTable<'_>,
    layout: UserMemoryLayout,
) -> bool {
    mapper
        .translate_addr(VirtAddr::new(layout.runtime_page_start()))
        .is_none()
}

pub(super) fn runtime_region_mapping_matches(
    mapper: &OffsetPageTable<'_>,
    address: u64,
    frame: PhysFrame,
) -> bool {
    let flags = PageTableFlags::PRESENT
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_EXECUTE;
    mapping_matches(mapper, address, frame, flags, PageTableFlags::HUGE_PAGE)
}

pub(super) fn runtime_region_unmapped(mapper: &OffsetPageTable<'_>, address: u64) -> bool {
    mapper.translate_addr(VirtAddr::new(address)).is_none()
}

fn supervisor_entry(entry: &PageTableEntry) -> bool {
    entry.is_unused() || !entry.flags().contains(PageTableFlags::USER_ACCESSIBLE)
}

fn stable_flags(entry: &PageTableEntry) -> PageTableFlags {
    entry
        .flags()
        .difference(PageTableFlags::ACCESSED | PageTableFlags::DIRTY)
}

fn mapping_matches(
    mapper: &OffsetPageTable<'_>,
    virtual_address: u64,
    frame: PhysFrame,
    required: PageTableFlags,
    forbidden: PageTableFlags,
) -> bool {
    match mapper.translate(VirtAddr::new(virtual_address)) {
        TranslateResult::Mapped {
            frame: MappedFrame::Size4KiB(actual),
            flags,
            ..
        } => actual == frame && flags.contains(required) && !flags.intersects(forbidden),
        _ => false,
    }
}
