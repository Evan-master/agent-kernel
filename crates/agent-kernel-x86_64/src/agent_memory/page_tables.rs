//! Agent P4 construction and isolation validation.
//!
//! This child module clones supervisor-only kernel P4 entries into one fresh
//! root, maps the dedicated Agent slot, and validates both roots before any CR3
//! transition. Shared lower tables are never modified through the Agent slot.

mod install;
mod lazy;
mod ownership;
mod runtime_page;
mod runtime_region;
mod validation;

use x86_64::{
    registers::control::Cr3,
    structures::paging::{PageTable, PhysFrame},
};

use agent_kernel_x86_64::{
    address_space::{AddressSpaceKind, AddressSpaceRoots, AGENT_PAGE_TABLE_FRAME_COUNT},
    user_memory::UserMemoryLayout,
};

use super::physical_pointer;

pub(super) use self::install::{install, install_reused};

pub(super) struct InstalledAgentPageTables {
    roots: AddressSpaceRoots,
    private_frames: [u64; AGENT_PAGE_TABLE_FRAME_COUNT],
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
