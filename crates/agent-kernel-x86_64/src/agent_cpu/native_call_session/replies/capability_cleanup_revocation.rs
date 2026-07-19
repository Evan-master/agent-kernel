//! Cleanup revocation acknowledgement for an authenticated Supervisor call.

use agent_kernel_core::{CapabilityId, ResourceId};
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_capability_cleanup_revocation(
        mut self,
        target: CapabilityId,
        resource: ResourceId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RevokeCapabilityForCleanup {
                    target: request_target,
                    ..
                } if request_target == target
            )
        })?;
        self.session
            .context
            .encode_capability_cleanup_revocation_reply(
                self.session.frame.frame_mut(),
                nonce,
                target,
                resource,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
