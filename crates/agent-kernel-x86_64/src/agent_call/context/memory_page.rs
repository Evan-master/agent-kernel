//! Canonical replies for native runtime memory-page Agent Calls.
//!
//! This context child validates fixed-width handles, addresses, and generation
//! values before writing operation-specific payloads over a canonical reply.

use agent_kernel_core::{MemoryCellId, ResourceId};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_ALLOCATE_MEMORY_PAGE, AGENT_CALL_INSPECT_MEMORY_PAGE,
        AGENT_CALL_MEMORY_PAGE_BYTES, AGENT_CALL_RELEASE_MEMORY_PAGE,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_memory_page_allocated_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        cell: MemoryCellId,
        virtual_base: u64,
        generation: u64,
    ) -> Result<(), AgentCallDecodeError> {
        if cell.raw() == 0
            || virtual_base == 0
            || !virtual_base.is_multiple_of(AGENT_CALL_MEMORY_PAGE_BYTES)
            || generation == 0
        {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_ALLOCATE_MEMORY_PAGE)?;
        frame.r10 = cell.raw();
        frame.r11 = virtual_base;
        frame.r12 = AGENT_CALL_MEMORY_PAGE_BYTES;
        frame.r13 = generation;
        Ok(())
    }

    pub fn encode_memory_page_inspected_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        cell: MemoryCellId,
        value: u64,
        generation: u64,
    ) -> Result<(), AgentCallDecodeError> {
        if cell.raw() == 0 || generation == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_INSPECT_MEMORY_PAGE)?;
        frame.r10 = cell.raw();
        frame.r11 = value;
        frame.r12 = generation;
        Ok(())
    }

    pub fn encode_memory_page_released_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        cell: MemoryCellId,
        resource: ResourceId,
        generation: u64,
    ) -> Result<(), AgentCallDecodeError> {
        if cell.raw() == 0 || resource.raw() == 0 || generation == 0 {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RELEASE_MEMORY_PAGE)?;
        frame.r10 = cell.raw();
        frame.r11 = resource.raw();
        frame.r12 = generation;
        Ok(())
    }
}
