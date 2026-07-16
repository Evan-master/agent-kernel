//! Physical memory and isolated address-space preparation for one Agent.
//!
//! This architecture-binary module owns Agent content frames and the kernel's
//! supervisor alias for the read-only call-release and quantum-generation
//! signal page. Its page-table child creates the distinct CR3 root and proves
//! that Agent virtual pages are kernel-unmapped.

mod frame_allocator;
mod page_tables;

use bootloader_api::BootInfo;
use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_x86_64::{
    address_space::{AddressSpaceRoots, AgentMemoryIdentity, AGENT_CONTENT_FRAME_COUNT},
    agent_image::VerifiedAgentImage,
    user_memory::{
        UserMemoryLayout, AGENT_CALL_RELEASE_OFFSET, AGENT_RESTART_GENERATION_OFFSET,
        FIRST_AGENT_RESTART_GENERATION, PAGE_BYTES, PHYSICAL_QUANTUM_GENERATION_OFFSET,
        STACK_PAGE_COUNT,
    },
};

use self::frame_allocator::BootFrameAllocator;

pub(crate) const PHYSICAL_MEMORY_OFFSET: u64 = 0xffff_8000_0000_0000;

pub(crate) struct PreparedAgentMemory {
    layout: UserMemoryLayout,
    signal_pointer: *mut u8,
    roots: AddressSpaceRoots,
    identity: AgentMemoryIdentity,
    entry_rip: u64,
}

impl PreparedAgentMemory {
    pub(crate) fn prepare(boot_info: &mut BootInfo, image: VerifiedAgentImage<'_>) -> Option<Self> {
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

        let signal_pointer = initialize_content(
            physical_offset,
            code_frame,
            signal_frame,
            &stack_frames,
            image.code(),
        )?;
        let layout = UserMemoryLayout::fixed();
        let entry_rip = layout
            .code_start()
            .checked_add(u64::from(image.entry_offset()))?;
        if !layout.contains_code(entry_rip) {
            return None;
        }
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
        let mut content_frames = [0; AGENT_CONTENT_FRAME_COUNT];
        content_frames[0] = code_frame.start_address().as_u64();
        content_frames[1] = signal_frame.start_address().as_u64();
        for (index, frame) in stack_frames.iter().enumerate() {
            content_frames[index + 2] = frame.start_address().as_u64();
        }
        let identity = AgentMemoryIdentity::new(roots.agent_root(), content_frames)?;

        Some(Self {
            layout,
            signal_pointer,
            roots,
            identity,
            entry_rip,
        })
    }

    pub(crate) const fn layout(&self) -> UserMemoryLayout {
        self.layout
    }

    pub(crate) const fn roots(&self) -> AddressSpaceRoots {
        self.roots
    }

    pub(crate) const fn entry_rip(&self) -> u64 {
        self.entry_rip
    }

    pub(crate) fn is_disjoint_from(&self, other: &Self) -> bool {
        self.identity.is_disjoint_from(other.identity)
    }

    pub(crate) fn kernel_address_space_active(&self) -> bool {
        page_tables::kernel_is_active(self.roots)
    }

    pub(crate) fn release_for_agent_call(&mut self) -> bool {
        // SAFETY: the kernel is active and owns this supervisor physical alias
        // of the exclusively allocated signal frame.
        if !self.agent_call_release_is_clear() {
            return false;
        }
        unsafe {
            let release = self.signal_pointer.add(AGENT_CALL_RELEASE_OFFSET);
            release.write_volatile(1);
            release.read_volatile() == 1
        }
    }

    pub(crate) fn signal_is_clear(&self) -> bool {
        // SAFETY: callers run at CPL0 under the kernel CR3; this pointer is the
        // supervisor physical alias of this Agent's exclusive signal frame.
        self.dispatch_signals_are_clear() && self.restart_generation() == 0
    }

    pub(crate) fn dispatch_signals_are_clear(&self) -> bool {
        self.agent_call_release_is_clear() && self.physical_quantum_generation() == 0
    }

    pub(crate) fn agent_call_is_released(&self) -> bool {
        // SAFETY: this is the same exclusive supervisor alias used to release
        // the Agent once for its complete returning call sequence.
        unsafe {
            self.signal_pointer
                .add(AGENT_CALL_RELEASE_OFFSET)
                .read_volatile()
                == 1
        }
    }

    pub(crate) fn record_physical_quantum_expiry(&mut self) -> Option<u8> {
        let generation = self.physical_quantum_generation().checked_add(1)?;
        // SAFETY: this byte is inside the exclusive signal frame and only the
        // single-core kernel writes it after validating an IRQ0 frame.
        unsafe {
            let pointer = self.signal_pointer.add(PHYSICAL_QUANTUM_GENERATION_OFFSET);
            pointer.write_volatile(generation);
            (pointer.read_volatile() == generation).then_some(generation)
        }
    }

    pub(crate) fn physical_quantum_generation(&self) -> u8 {
        // SAFETY: this fixed offset is inside the exclusive signal frame.
        unsafe {
            self.signal_pointer
                .add(PHYSICAL_QUANTUM_GENERATION_OFFSET)
                .read_volatile()
        }
    }

    pub(crate) fn restart_generation(&self) -> u8 {
        // SAFETY: this fixed offset is inside the exclusive signal frame.
        unsafe {
            self.signal_pointer
                .add(AGENT_RESTART_GENERATION_OFFSET)
                .read_volatile()
        }
    }

    pub(crate) fn reset_for_first_restart(self) -> Option<Self> {
        if !self.kernel_address_space_active() || self.restart_generation() != 0 {
            return None;
        }
        let frames = self.identity.content_frames();
        let signal_frame = PhysFrame::from_start_address(PhysAddr::new(frames[1])).ok()?;
        if physical_pointer(PHYSICAL_MEMORY_OFFSET, signal_frame)? != self.signal_pointer
            || !clear_page(self.signal_pointer)
        {
            return None;
        }
        for frame_address in &frames[2..] {
            let frame = PhysFrame::from_start_address(PhysAddr::new(*frame_address)).ok()?;
            if !clear_page(physical_pointer(PHYSICAL_MEMORY_OFFSET, frame)?) {
                return None;
            }
        }
        // SAFETY: reset retained exclusive ownership of the signal frame and
        // ring 3 maps it read-only.
        unsafe {
            self.signal_pointer
                .add(AGENT_RESTART_GENERATION_OFFSET)
                .write_volatile(FIRST_AGENT_RESTART_GENERATION);
        }
        (self.dispatch_signals_are_clear()
            && self.restart_generation() == FIRST_AGENT_RESTART_GENERATION)
            .then_some(self)
    }

    fn agent_call_release_is_clear(&self) -> bool {
        // SAFETY: this fixed offset is inside the exclusive signal frame.
        unsafe {
            self.signal_pointer
                .add(AGENT_CALL_RELEASE_OFFSET)
                .read_volatile()
                == 0
        }
    }
}

fn clear_page(pointer: *mut u8) -> bool {
    // SAFETY: callers pass a supervisor alias for an exclusively owned Agent
    // signal or stack frame while the kernel address space is active.
    unsafe {
        pointer.write_bytes(0, PAGE_BYTES as usize);
    }
    let mut offset = 0;
    while offset < PAGE_BYTES as usize {
        // SAFETY: `offset` remains inside the same exclusive page.
        if unsafe { pointer.add(offset).read_volatile() } != 0 {
            return false;
        }
        offset += 1;
    }
    true
}

fn initialize_content(
    physical_offset: u64,
    code_frame: PhysFrame,
    signal_frame: PhysFrame,
    stack_frames: &[PhysFrame; STACK_PAGE_COUNT],
    code: &[u8],
) -> Option<*mut u8> {
    if code.is_empty() || code.len() > PAGE_BYTES as usize {
        return None;
    }
    let code_pointer = physical_pointer(physical_offset, code_frame)?;
    let signal_pointer = physical_pointer(physical_offset, signal_frame)?;
    // SAFETY: all frames were just removed from Usable memory and are
    // exclusively owned by this preparation operation.
    unsafe {
        code_pointer.write_bytes(0, PAGE_BYTES as usize);
        code.as_ptr()
            .copy_to_nonoverlapping(code_pointer, code.len());
        signal_pointer.write_bytes(0, PAGE_BYTES as usize);
        for frame in stack_frames {
            physical_pointer(physical_offset, *frame)?.write_bytes(0, PAGE_BYTES as usize);
        }
    }
    for (offset, expected) in code.iter().copied().enumerate() {
        // SAFETY: the code frame remains exclusively owned and supervisor-mapped.
        if unsafe { code_pointer.add(offset).read_volatile() } != expected {
            return None;
        }
    }
    Some(signal_pointer)
}

pub(super) fn physical_pointer(offset: u64, frame: PhysFrame) -> Option<*mut u8> {
    let address = offset.checked_add(frame.start_address().as_u64())?;
    Some(address as usize as *mut u8)
}
