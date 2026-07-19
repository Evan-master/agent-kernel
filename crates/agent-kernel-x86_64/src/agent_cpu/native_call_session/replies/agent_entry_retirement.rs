//! Agent Entry retirement acknowledgement for an authenticated Supervisor call.

use agent_kernel_core::AgentEntryRetirement;
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_agent_entry_retirement(
        mut self,
        receipt: AgentEntryRetirement,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RetireAgentEntry { target, .. }
                    if target == receipt.agent()
            )
        })?;
        self.session
            .context
            .encode_agent_entry_retirement_reply(
                self.session.frame.frame_mut(),
                nonce,
                receipt.agent(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
