//! Agent Image record retirement acknowledgement for an authenticated call.

use agent_kernel_core::AgentImageRecordRetirement;
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_agent_image_record_retirement(
        mut self,
        receipt: AgentImageRecordRetirement,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RetireAgentImageRecord { target, .. }
                    if target == receipt.image()
            )
        })?;
        let record = receipt.record();
        self.session
            .context
            .encode_agent_image_record_retirement_reply(
                self.session.frame.frame_mut(),
                nonce,
                record.id,
                record.resource,
                record.owner,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
