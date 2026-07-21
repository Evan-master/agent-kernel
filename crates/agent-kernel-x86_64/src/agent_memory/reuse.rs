//! Rebuild of one native Agent address space from reclaimed physical frames.
//!
//! This bare-metal Agent-memory child consumes a complete allocation owner,
//! verifies zeroed physical contents, restores the fixed private page-table
//! hierarchy, and returns the same prepared-memory type used by boot allocation.

use x86_64::{structures::paging::PhysFrame, PhysAddr};

use agent_kernel_x86_64::{
    address_space::{AgentMemoryIdentity, AGENT_CODE_PAGE_CAPACITY},
    address_space_reclamation::AllocatedAddressSpaceFrames,
    agent_image::VerifiedAgentImage,
    runtime_page::RuntimePageLedger,
    runtime_region::{RuntimeRegionLedger, RuntimeRegionObservationLog},
    user_memory::{UserMemoryLayout, STACK_PAGE_COUNT},
};

use super::{
    initialize_content, page_is_zero, page_tables, physical_pointer, PreparedAgentMemory,
    PHYSICAL_MEMORY_OFFSET,
};

impl PreparedAgentMemory {
    // Failed reconstruction returns all physical frames to the caller intact.
    #[allow(clippy::result_large_err)]
    pub(crate) fn prepare_reused(
        frames: AllocatedAddressSpaceFrames,
        image: VerifiedAgentImage<'_>,
    ) -> Result<Self, AllocatedAddressSpaceFrames> {
        let agent = frames.agent();
        let identity = frames.identity();
        let Some(memory) = Self::build_reused(agent, identity, image) else {
            return Err(frames);
        };
        if memory.identity != identity {
            return Err(frames);
        }
        let _transferred_identity = frames.into_identity();
        Ok(memory)
    }

    fn build_reused(
        agent: agent_kernel_core::AgentId,
        identity: AgentMemoryIdentity,
        image: VerifiedAgentImage<'_>,
    ) -> Option<Self> {
        if identity.code_page_count() != image.code_page_count()
            || !identity.owned_frames().into_iter().all(frame_is_zero)
        {
            return None;
        }

        let code_addresses = identity.code_frames();
        let first_code_frame = physical_frame(code_addresses[0])?;
        let mut code_frame_storage = [first_code_frame; AGENT_CODE_PAGE_CAPACITY];
        for (slot, address) in code_frame_storage
            .iter_mut()
            .zip(code_addresses.iter().copied())
        {
            *slot = physical_frame(address)?;
        }
        let code_frames = &code_frame_storage[..identity.code_page_count()];
        let signal_frame = physical_frame(identity.signal_frame())?;
        let stack_addresses = identity.stack_frames();
        let first_stack_frame = physical_frame(stack_addresses[0])?;
        let mut stack_frames = [first_stack_frame; STACK_PAGE_COUNT];
        for (slot, address) in stack_frames.iter_mut().zip(stack_addresses) {
            *slot = physical_frame(address)?;
        }
        let lazy_data_frame = physical_frame(identity.lazy_data_frame())?;
        let call_data_frame = physical_frame(identity.call_data_frame())?;
        let (signal_pointer, lazy_data_pointer, call_data_pointer) = initialize_content(
            PHYSICAL_MEMORY_OFFSET,
            code_frames,
            signal_frame,
            &stack_frames,
            lazy_data_frame,
            call_data_frame,
            image.code(),
        )?;
        let layout = UserMemoryLayout::fixed();
        let entry_rip = layout
            .code_start()
            .checked_add(u64::from(image.entry_offset()))?;
        if !layout.contains_code(entry_rip) {
            return None;
        }
        let installed = page_tables::install_reused(
            PHYSICAL_MEMORY_OFFSET,
            identity.page_table_frames(),
            layout,
            code_frames,
            signal_frame,
            &stack_frames,
            lazy_data_frame,
            call_data_frame,
        )?;
        let roots = installed.roots();
        if installed.private_frames() != identity.page_table_frames()
            || identity.root() != roots.agent_root()
            || !page_tables::kernel_is_active(roots)
        {
            return None;
        }

        Some(Self {
            allocated_for: Some(agent),
            layout,
            signal_pointer,
            lazy_data_pointer,
            call_data_pointer,
            roots,
            identity,
            entry_rip,
            runtime_page: RuntimePageLedger::new(),
            runtime_page_observation: None,
            runtime_regions: RuntimeRegionLedger::new(),
            runtime_region_observations: RuntimeRegionObservationLog::new(),
        })
    }

    pub(crate) const fn identity(&self) -> AgentMemoryIdentity {
        self.identity
    }
}

fn physical_frame(address: u64) -> Option<PhysFrame> {
    PhysFrame::from_start_address(PhysAddr::new(address)).ok()
}

fn frame_is_zero(address: u64) -> bool {
    physical_frame(address)
        .and_then(|frame| physical_pointer(PHYSICAL_MEMORY_OFFSET, frame))
        .is_some_and(page_is_zero)
}
