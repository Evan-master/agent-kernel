//! Owned-frame acknowledgements for native runtime memory-region calls.
//!
//! Reply encoding runs after semantic, virtual, and physical ownership changes
//! have all committed under the authenticated Agent Call context.

use agent_kernel_core::{MemoryCellId, ResourceId};
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_memory_region_allocated(
        mut self,
        cell: MemoryCellId,
        virtual_base: u64,
        page_count: u64,
        generation: u64,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::AllocateMemoryRegion { .. })
        })?;
        self.session
            .context
            .encode_memory_region_allocated_reply(
                self.session.frame.frame_mut(),
                nonce,
                cell,
                virtual_base,
                page_count,
                generation,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_memory_region_inspected(
        mut self,
        cell: MemoryCellId,
        first: u64,
        last: u64,
        page_count: u64,
        generation: u64,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::InspectMemoryRegion { .. })
        })?;
        self.session
            .context
            .encode_memory_region_inspected_reply(
                self.session.frame.frame_mut(),
                nonce,
                cell,
                first,
                last,
                page_count,
                generation,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_memory_region_released(
        mut self,
        cell: MemoryCellId,
        resource: ResourceId,
        page_count: u64,
        generation: u64,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::ReleaseMemoryRegion { .. })
        })?;
        self.session
            .context
            .encode_memory_region_released_reply(
                self.session.frame.frame_mut(),
                nonce,
                cell,
                resource,
                page_count,
                generation,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
