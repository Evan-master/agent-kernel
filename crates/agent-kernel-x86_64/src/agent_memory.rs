//! Physical memory and isolated address-space preparation for one Agent.
//!
//! This architecture-binary module owns Agent content frames and the kernel's
//! supervisor alias for the read-only call-release, quantum-generation, and
//! restart-generation signal page. Its page-table child creates the distinct
//! CR3 root and proves that Agent virtual pages are kernel-unmapped.

mod address_space_reclamation;
mod frame_allocator;
mod page_tables;
mod reclamation;
mod reuse;
mod runtime_page;
mod runtime_pool;
mod runtime_region;

use bootloader_api::BootInfo;
use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_x86_64::{
    address_space::{AddressSpaceRoots, AgentMemoryIdentity, AGENT_CONTENT_FRAME_COUNT},
    agent_image::VerifiedAgentImage,
    runtime_page::RuntimePageLedger,
    runtime_region::{RuntimeRegionLedger, RuntimeRegionObservationLog},
    user_memory::{
        UserMemoryLayout, AGENT_CALL_RELEASE_OFFSET, AGENT_RESTART_GENERATION_OFFSET,
        MAX_AGENT_RESTART_GENERATION, PAGE_BYTES, PHYSICAL_QUANTUM_GENERATION_OFFSET,
        STACK_PAGE_COUNT,
    },
};

use self::frame_allocator::BootFrameAllocator;

pub(crate) use self::{
    address_space_reclamation::{
        NativeAddressSpaceFramePool, ReclaimedAgentAddressSpace, NATIVE_ADDRESS_SPACE_CAPACITY,
        NATIVE_ADDRESS_SPACE_FRAME_CAPACITY,
    },
    runtime_pool::{RuntimeMemoryPool, RuntimePhysicalFrameSet},
};

pub(crate) const PHYSICAL_MEMORY_OFFSET: u64 = 0xffff_8000_0000_0000;

pub(crate) struct PreparedAgentMemory {
    layout: UserMemoryLayout,
    signal_pointer: *mut u8,
    lazy_data_pointer: *mut u8,
    roots: AddressSpaceRoots,
    identity: AgentMemoryIdentity,
    entry_rip: u64,
    runtime_page: RuntimePageLedger,
    runtime_page_observation: Option<u64>,
    runtime_regions: RuntimeRegionLedger,
    runtime_region_observations: RuntimeRegionObservationLog,
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
        let lazy_data_frame = allocator.allocate()?;

        let (signal_pointer, lazy_data_pointer) = initialize_content(
            physical_offset,
            code_frame,
            signal_frame,
            &stack_frames,
            lazy_data_frame,
            image.code(),
        )?;
        let layout = UserMemoryLayout::fixed();
        let entry_rip = layout
            .code_start()
            .checked_add(u64::from(image.entry_offset()))?;
        if !layout.contains_code(entry_rip) {
            return None;
        }
        let installed = page_tables::install(
            physical_offset,
            &mut allocator,
            layout,
            code_frame,
            signal_frame,
            &stack_frames,
            lazy_data_frame,
        )?;
        let roots = installed.roots();
        if !page_tables::kernel_is_active(roots) {
            return None;
        }
        let mut content_frames = [0; AGENT_CONTENT_FRAME_COUNT];
        content_frames[0] = code_frame.start_address().as_u64();
        content_frames[1] = signal_frame.start_address().as_u64();
        for (index, frame) in stack_frames.iter().enumerate() {
            content_frames[index + 2] = frame.start_address().as_u64();
        }
        content_frames[STACK_PAGE_COUNT + 2] = lazy_data_frame.start_address().as_u64();
        let identity = AgentMemoryIdentity::new(installed.private_frames(), content_frames)?;
        if identity.root() != roots.agent_root() {
            return None;
        }

        Some(Self {
            layout,
            signal_pointer,
            lazy_data_pointer,
            roots,
            identity,
            entry_rip,
            runtime_page: RuntimePageLedger::new(),
            runtime_page_observation: None,
            runtime_regions: RuntimeRegionLedger::new(),
            runtime_region_observations: RuntimeRegionObservationLog::new(),
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

    pub(crate) fn lazy_data_byte(&self) -> u8 {
        // SAFETY: this pointer is the supervisor alias of the retained private
        // frame, whether or not the Agent leaf mapping has been activated.
        unsafe { self.lazy_data_pointer.read_volatile() }
    }

    pub(crate) fn activate_lazy_data_page(&mut self, fault_address: u64) -> Option<()> {
        if !self.kernel_address_space_active()
            || fault_address != self.layout.lazy_data_start()
            || self.lazy_data_byte() != 0
        {
            return None;
        }
        let frame_address = self.identity.content_frames()[STACK_PAGE_COUNT + 2];
        let frame = PhysFrame::from_start_address(PhysAddr::new(frame_address)).ok()?;
        if physical_pointer(PHYSICAL_MEMORY_OFFSET, frame)? != self.lazy_data_pointer {
            return None;
        }
        page_tables::activate_lazy_data(PHYSICAL_MEMORY_OFFSET, self.roots, self.layout, frame)
    }

    pub(crate) fn reset_for_next_restart(self) -> Option<(Self, u8)> {
        if !self.kernel_address_space_active()
            || !self.runtime_page.is_available()
            || !self.runtime_regions.is_clear()
        {
            return None;
        }
        let next_generation = self.restart_generation().checked_add(1)?;
        if next_generation > MAX_AGENT_RESTART_GENERATION {
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
                .write_volatile(next_generation);
        }
        (self.dispatch_signals_are_clear() && self.restart_generation() == next_generation)
            .then_some((self, next_generation))
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

pub(super) fn clear_page(pointer: *mut u8) -> bool {
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

pub(super) fn page_is_zero(pointer: *mut u8) -> bool {
    let mut offset = 0;
    while offset < PAGE_BYTES as usize {
        // SAFETY: callers bind this pointer to one exclusive 4 KiB frame.
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
    lazy_data_frame: PhysFrame,
    code: &[u8],
) -> Option<(*mut u8, *mut u8)> {
    if code.is_empty() || code.len() > PAGE_BYTES as usize {
        return None;
    }
    let code_pointer = physical_pointer(physical_offset, code_frame)?;
    let signal_pointer = physical_pointer(physical_offset, signal_frame)?;
    let lazy_data_pointer = physical_pointer(physical_offset, lazy_data_frame)?;
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
        lazy_data_pointer.write_bytes(0, PAGE_BYTES as usize);
    }
    for (offset, expected) in code.iter().copied().enumerate() {
        // SAFETY: the code frame remains exclusively owned and supervisor-mapped.
        if unsafe { code_pointer.add(offset).read_volatile() } != expected {
            return None;
        }
    }
    Some((signal_pointer, lazy_data_pointer))
}

pub(super) fn physical_pointer(offset: u64, frame: PhysFrame) -> Option<*mut u8> {
    let address = offset.checked_add(frame.start_address().as_u64())?;
    Some(address as usize as *mut u8)
}
