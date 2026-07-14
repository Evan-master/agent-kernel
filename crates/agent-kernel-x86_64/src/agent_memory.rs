//! Fixed physical allocation and user-page mapping for one ring-3 Agent.
//!
//! This architecture-binary module consumes only BootInfo Usable frames,
//! writes the host-tested proof program through the supervisor physical
//! window, and maps least-authority user pages in the active page table.

mod frame_allocator;

use bootloader_api::BootInfo;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper::{MappedFrame, Translate, TranslateResult},
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use agent_kernel_x86_64::user_memory::{
    agent_proof_program, UserMemoryLayout, PAGE_BYTES, STACK_PAGE_COUNT,
};

use self::frame_allocator::BootFrameAllocator;

pub(crate) const PHYSICAL_MEMORY_OFFSET: u64 = 0xffff_8000_0000_0000;

pub(crate) struct PreparedUserMemory {
    layout: UserMemoryLayout,
    signal_pointer: *mut u8,
}

impl PreparedUserMemory {
    pub(crate) fn prepare(boot_info: &mut BootInfo) -> Option<Self> {
        let physical_offset = boot_info.physical_memory_offset.into_option()?;
        if physical_offset != PHYSICAL_MEMORY_OFFSET {
            return None;
        }
        let mut allocator = BootFrameAllocator::new(&mut boot_info.memory_regions);
        let code_frame = allocator.allocate_frame()?;
        let signal_frame = allocator.allocate_frame()?;
        let zero_frame = PhysFrame::from_start_address(PhysAddr::new(0)).ok()?;
        let mut stack_frames = [zero_frame; STACK_PAGE_COUNT];
        for frame in &mut stack_frames {
            *frame = allocator.allocate_frame()?;
        }

        let program = agent_proof_program();
        let code_pointer = physical_pointer(physical_offset, code_frame)?;
        let signal_pointer = physical_pointer(physical_offset, signal_frame)?;
        // SAFETY: all frames were just removed from Usable memory and are
        // exclusively owned by this mapping operation.
        unsafe {
            code_pointer.write_bytes(0, PAGE_BYTES as usize);
            program
                .as_ptr()
                .copy_to_nonoverlapping(code_pointer, program.len());
            signal_pointer.write_bytes(0, PAGE_BYTES as usize);
            for frame in stack_frames {
                physical_pointer(physical_offset, frame)?.write_bytes(0, PAGE_BYTES as usize);
            }
        }

        let level_4_frame = Cr3::read().0;
        let level_4_pointer = physical_pointer(physical_offset, level_4_frame)?.cast::<PageTable>();
        // SAFETY: CR3 identifies the active level-4 table and the configured
        // physical window maps every page-table frame at this exact offset.
        let mut mapper = unsafe {
            OffsetPageTable::new(&mut *level_4_pointer, VirtAddr::new(PHYSICAL_MEMORY_OFFSET))
        };
        let layout = UserMemoryLayout::fixed();
        let code_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        let signal_flags =
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::NO_EXECUTE;
        let stack_flags = signal_flags | PageTableFlags::WRITABLE;

        map_page(
            &mut mapper,
            &mut allocator,
            layout.code_start(),
            code_frame,
            code_flags,
        )?;
        map_page(
            &mut mapper,
            &mut allocator,
            layout.signal_start(),
            signal_frame,
            signal_flags,
        )?;
        for (index, frame) in stack_frames.into_iter().enumerate() {
            map_page(
                &mut mapper,
                &mut allocator,
                layout.stack_bottom() + PAGE_BYTES * index as u64,
                frame,
                stack_flags,
            )?;
        }
        for (index, frame) in stack_frames.iter().enumerate() {
            if !mapping_matches(
                &mapper,
                layout.stack_bottom() + PAGE_BYTES * index as u64,
                *frame,
                stack_flags,
                PageTableFlags::empty(),
            ) {
                return None;
            }
        }

        if !mapping_matches(
            &mapper,
            layout.code_start(),
            code_frame,
            code_flags,
            PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
        ) || !mapping_matches(
            &mapper,
            layout.signal_start(),
            signal_frame,
            signal_flags,
            PageTableFlags::WRITABLE,
        ) || mapper
            .translate_addr(VirtAddr::new(layout.guard_start()))
            .is_some()
            || unsafe { signal_pointer.read_volatile() } != 0
        {
            return None;
        }

        Some(Self {
            layout,
            signal_pointer,
        })
    }

    pub(crate) const fn layout(&self) -> UserMemoryLayout {
        self.layout
    }

    pub(crate) fn release_for_agent_call(&mut self) -> bool {
        // SAFETY: the kernel owns execution and this pointer names the
        // supervisor alias of the exclusively allocated signal frame.
        unsafe {
            self.signal_pointer.write_volatile(1);
            self.signal_pointer.read_volatile() == 1
        }
    }
}

fn physical_pointer(offset: u64, frame: PhysFrame<Size4KiB>) -> Option<*mut u8> {
    let address = offset.checked_add(frame.start_address().as_u64())?;
    Some(address as usize as *mut u8)
}

fn map_page(
    mapper: &mut OffsetPageTable<'_>,
    allocator: &mut BootFrameAllocator<'_>,
    virtual_address: u64,
    frame: PhysFrame<Size4KiB>,
    flags: PageTableFlags,
) -> Option<()> {
    let page = Page::from_start_address(VirtAddr::new(virtual_address)).ok()?;
    // SAFETY: each virtual page and physical frame is fresh and exclusively
    // owned; flags encode the intended user authority for that page.
    unsafe {
        mapper.map_to(page, frame, flags, allocator).ok()?.flush();
    }
    Some(())
}

fn mapping_matches(
    mapper: &OffsetPageTable<'_>,
    virtual_address: u64,
    frame: PhysFrame<Size4KiB>,
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
