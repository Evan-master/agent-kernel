//! Owned-frame acknowledgements for native runtime memory-page calls.
//!
//! This CPU-session child authenticates each operation against the retained
//! nonce and writes the canonical architecture reply after semantic and
//! physical transitions have both succeeded.

use agent_kernel_core::{MemoryCellId, ResourceId};
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_memory_page_allocated(
        mut self,
        cell: MemoryCellId,
        virtual_base: u64,
        generation: u64,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::AllocateMemoryPage { .. })
        })?;
        self.session
            .context
            .encode_memory_page_allocated_reply(
                self.session.frame.frame_mut(),
                nonce,
                cell,
                virtual_base,
                generation,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_memory_page_inspected(
        mut self,
        cell: MemoryCellId,
        value: u64,
        generation: u64,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::InspectMemoryPage { .. })
        })?;
        self.session
            .context
            .encode_memory_page_inspected_reply(
                self.session.frame.frame_mut(),
                nonce,
                cell,
                value,
                generation,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_memory_page_released(
        mut self,
        cell: MemoryCellId,
        resource: ResourceId,
        generation: u64,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::ReleaseMemoryPage { .. })
        })?;
        self.session
            .context
            .encode_memory_page_released_reply(
                self.session.frame.frame_mut(),
                nonce,
                cell,
                resource,
                generation,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
