//! Resource record retirement acknowledgement for an authenticated call.

use agent_kernel_core::ResourceRecordRetirement;
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_resource_record_retirement(
        mut self,
        receipt: ResourceRecordRetirement,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RetireResourceRecord { target, .. }
                    if target == receipt.resource()
            )
        })?;
        self.session
            .context
            .encode_resource_record_retirement_reply(
                self.session.frame.frame_mut(),
                nonce,
                receipt.record(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
