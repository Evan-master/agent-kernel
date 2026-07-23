//! Irreversible removal of one completed Agent's private P4 slot.
//!
//! The root frame and every mapped content frame remain owned until a matching
//! cross-CPU TLB completion permits physical reclamation.

use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_x86_64::address_space::{AddressSpaceRoots, AGENT_P4_INDEX};

use super::table_pointer;

pub(super) fn remove_agent_slot(physical_offset: u64, roots: AddressSpaceRoots) -> Option<()> {
    let root = root_pointer(physical_offset, roots)?;
    // SAFETY: the completed Agent is no longer schedulable, the kernel CR3 is
    // active, and this private P4 entry belongs only to the retained root.
    unsafe {
        let entry = &mut (&mut *root)[AGENT_P4_INDEX];
        if entry.is_unused() {
            return None;
        }
        entry.set_unused();
    }
    agent_slot_removed(physical_offset, roots).then_some(())
}

pub(super) fn agent_slot_removed(physical_offset: u64, roots: AddressSpaceRoots) -> bool {
    let Some(root) = root_pointer(physical_offset, roots) else {
        return false;
    };
    // SAFETY: the retained root frame remains exclusively owned and readable.
    unsafe { (&*root)[AGENT_P4_INDEX].is_unused() }
}

fn root_pointer(
    physical_offset: u64,
    roots: AddressSpaceRoots,
) -> Option<*mut x86_64::structures::paging::PageTable> {
    let frame = PhysFrame::from_start_address(PhysAddr::new(roots.agent_root())).ok()?;
    table_pointer(physical_offset, frame)
}
