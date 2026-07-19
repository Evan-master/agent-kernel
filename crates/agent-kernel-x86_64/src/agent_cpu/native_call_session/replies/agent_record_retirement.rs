//! Agent Record retirement acknowledgement for an authenticated call.

use agent_kernel_core::AgentRecordRetirement;
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_agent_record_retirement(
        mut self,
        receipt: AgentRecordRetirement,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RetireAgentRecord { target, .. }
                    if target == receipt.agent()
            )
        })?;
        self.session
            .context
            .encode_agent_record_retirement_reply(
                self.session.frame.frame_mut(),
                nonce,
                receipt.agent(),
                receipt.management_resource(),
                receipt.retired_floor(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
