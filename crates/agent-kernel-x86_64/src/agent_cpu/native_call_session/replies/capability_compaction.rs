//! Capability compaction acknowledgement for an authenticated Supervisor call.

use agent_kernel_core::CapabilityCompaction;
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_capability_compaction(
        mut self,
        receipt: CapabilityCompaction,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::CompactCapability { target, .. }
                    if target == receipt.capability()
            )
        })?;
        self.session
            .context
            .encode_capability_compaction_reply(
                self.session.frame.frame_mut(),
                nonce,
                receipt.capability(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
