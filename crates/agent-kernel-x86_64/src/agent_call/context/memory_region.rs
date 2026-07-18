//! Canonical replies for native runtime memory-region Agent Calls.
//!
//! This context child validates page-aligned addresses, bounded lengths,
//! handles, and generations before writing operation-specific payloads over
//! the authenticated common reply.

use agent_kernel_core::{MemoryCellId, ResourceId};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_ALLOCATE_MEMORY_REGION, AGENT_CALL_INSPECT_MEMORY_REGION,
        AGENT_CALL_MEMORY_REGION_MAX_PAGES, AGENT_CALL_MEMORY_REGION_PAGE_BYTES,
        AGENT_CALL_RELEASE_MEMORY_REGION,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_memory_region_allocated_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        cell: MemoryCellId,
        virtual_base: u64,
        page_count: u64,
        generation: u64,
    ) -> Result<(), AgentCallDecodeError> {
        let byte_length = valid_region(cell, virtual_base, page_count, generation)?;
        self.encode_reply(frame, nonce, AGENT_CALL_ALLOCATE_MEMORY_REGION)?;
        frame.r10 = cell.raw();
        frame.r11 = virtual_base;
        frame.r12 = byte_length;
        frame.r13 = page_count;
        frame.r14 = generation;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn encode_memory_region_inspected_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        cell: MemoryCellId,
        first_value: u64,
        last_value: u64,
        page_count: u64,
        generation: u64,
    ) -> Result<(), AgentCallDecodeError> {
        if cell.raw() == 0 || !valid_page_count(page_count) || generation == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_INSPECT_MEMORY_REGION)?;
        frame.r10 = cell.raw();
        frame.r11 = first_value;
        frame.r12 = last_value;
        frame.r13 = page_count;
        frame.r14 = generation;
        Ok(())
    }

    pub fn encode_memory_region_released_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        cell: MemoryCellId,
        resource: ResourceId,
        page_count: u64,
        generation: u64,
    ) -> Result<(), AgentCallDecodeError> {
        if cell.raw() == 0
            || resource.raw() == 0
            || !valid_page_count(page_count)
            || generation == 0
        {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RELEASE_MEMORY_REGION)?;
        frame.r10 = cell.raw();
        frame.r11 = resource.raw();
        frame.r12 = page_count;
        frame.r13 = generation;
        Ok(())
    }
}

fn valid_region(
    cell: MemoryCellId,
    virtual_base: u64,
    page_count: u64,
    generation: u64,
) -> Result<u64, AgentCallDecodeError> {
    if cell.raw() == 0
        || virtual_base == 0
        || !virtual_base.is_multiple_of(AGENT_CALL_MEMORY_REGION_PAGE_BYTES)
        || !valid_page_count(page_count)
        || generation == 0
    {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    page_count
        .checked_mul(AGENT_CALL_MEMORY_REGION_PAGE_BYTES)
        .ok_or(AgentCallDecodeError::InvalidPayload)
}

const fn valid_page_count(page_count: u64) -> bool {
    page_count != 0 && page_count <= AGENT_CALL_MEMORY_REGION_MAX_PAGES
}
