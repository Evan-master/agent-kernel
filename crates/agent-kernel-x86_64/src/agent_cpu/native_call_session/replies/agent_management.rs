//! Managed Agent acknowledgements for an owned native call session.
//!
//! This CPU-session child authenticates the pending request and maps its typed
//! operation to the canonical x86 reply encoder after semantic mutation has
//! completed successfully.

use agent_kernel_core::{AgentId, AgentStatus, ResourceId};
use agent_kernel_x86_64::agent_call::{
    AgentCallRequest, AGENT_CALL_REGISTER_MANAGED_AGENT, AGENT_CALL_RESUME_MANAGED_AGENT,
    AGENT_CALL_RETIRE_MANAGED_AGENT, AGENT_CALL_SUSPEND_MANAGED_AGENT,
};

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_agent_management(
        mut self,
        target: AgentId,
        resource: ResourceId,
        status: AgentStatus,
    ) -> Option<ResumableAgentCpu> {
        let request = self.authenticated_request()?;
        let nonce = self.session.progress.nonce?;
        let operation = match request {
            AgentCallRequest::RegisterManagedAgent { .. } => AGENT_CALL_REGISTER_MANAGED_AGENT,
            AgentCallRequest::SuspendManagedAgent { .. } => AGENT_CALL_SUSPEND_MANAGED_AGENT,
            AgentCallRequest::ResumeManagedAgent { .. } => AGENT_CALL_RESUME_MANAGED_AGENT,
            AgentCallRequest::RetireManagedAgent { .. } => AGENT_CALL_RETIRE_MANAGED_AGENT,
            _ => return None,
        };
        self.session
            .context
            .encode_agent_management_reply(
                self.session.frame.frame_mut(),
                nonce,
                operation,
                target,
                resource,
                status,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
