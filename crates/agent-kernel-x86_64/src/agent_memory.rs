//! Physical memory and isolated address-space preparation for one Agent.
//!
//! This architecture-binary module owns Agent content frames and the kernel's
//! supervisor alias for the signal page. Its page-table child creates the
//! distinct CR3 root and proves that Agent virtual pages are kernel-unmapped.

mod frame_allocator;
mod page_tables;

use bootloader_api::BootInfo;
use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_x86_64::{
    address_space::AddressSpaceRoots,
    user_memory::{agent_proof_program, UserMemoryLayout, PAGE_BYTES, STACK_PAGE_COUNT},
};

use self::frame_allocator::BootFrameAllocator;

pub(crate) const PHYSICAL_MEMORY_OFFSET: u64 = 0xffff_8000_0000_0000;

pub(crate) struct PreparedAgentMemory {
    layout: UserMemoryLayout,
    signal_pointer: *mut u8,
    roots: AddressSpaceRoots,
}

impl PreparedAgentMemory {
    pub(crate) fn prepare(boot_info: &mut BootInfo) -> Option<Self> {
        let physical_offset = boot_info.physical_memory_offset.into_option()?;
        if physical_offset != PHYSICAL_MEMORY_OFFSET {
            return None;
        }
        let mut allocator = BootFrameAllocator::new(&mut boot_info.memory_regions);
        let code_frame = allocator.allocate()?;
        let signal_frame = allocator.allocate()?;
        let zero_frame = PhysFrame::from_start_address(PhysAddr::new(0)).ok()?;
        let mut stack_frames = [zero_frame; STACK_PAGE_COUNT];
        for frame in &mut stack_frames {
            *frame = allocator.allocate()?;
        }

        let signal_pointer =
            initialize_content(physical_offset, code_frame, signal_frame, &stack_frames)?;
        let layout = UserMemoryLayout::fixed();
        let roots = page_tables::install(
            physical_offset,
            &mut allocator,
            layout,
            code_frame,
            signal_frame,
            &stack_frames,
        )?;
        if !page_tables::kernel_is_active(roots) {
            return None;
        }

        Some(Self {
            layout,
            signal_pointer,
            roots,
        })
    }

    pub(crate) const fn layout(&self) -> UserMemoryLayout {
        self.layout
    }

    pub(crate) const fn roots(&self) -> AddressSpaceRoots {
        self.roots
    }

    pub(crate) fn kernel_address_space_active(&self) -> bool {
        page_tables::kernel_is_active(self.roots)
    }

    pub(crate) fn release_for_agent_call(&mut self) -> bool {
        // SAFETY: the kernel is active and owns this supervisor physical alias
        // of the exclusively allocated signal frame.
        unsafe {
            self.signal_pointer.write_volatile(1);
            self.signal_pointer.read_volatile() == 1
        }
    }
}

fn initialize_content(
    physical_offset: u64,
    code_frame: PhysFrame,
    signal_frame: PhysFrame,
    stack_frames: &[PhysFrame; STACK_PAGE_COUNT],
) -> Option<*mut u8> {
    let program = agent_proof_program();
    let code_pointer = physical_pointer(physical_offset, code_frame)?;
    let signal_pointer = physical_pointer(physical_offset, signal_frame)?;
    // SAFETY: all frames were just removed from Usable memory and are
    // exclusively owned by this preparation operation.
    unsafe {
        code_pointer.write_bytes(0, PAGE_BYTES as usize);
        program
            .as_ptr()
            .copy_to_nonoverlapping(code_pointer, program.len());
        signal_pointer.write_bytes(0, PAGE_BYTES as usize);
        for frame in stack_frames {
            physical_pointer(physical_offset, *frame)?.write_bytes(0, PAGE_BYTES as usize);
        }
    }
    Some(signal_pointer)
}

pub(super) fn physical_pointer(offset: u64, frame: PhysFrame) -> Option<*mut u8> {
    let address = offset.checked_add(frame.start_address().as_u64())?;
    Some(address as usize as *mut u8)
}
