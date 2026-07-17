//! Intent and Task lifecycle acknowledgements for native call sessions.
//!
//! Each transition requires an authenticated pending request and writes the
//! matching canonical reply into the owned privilege frame.

use agent_kernel_core::{AgentId, CapabilityId, IntentId, TaskId};
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_intent_declared(
        mut self,
        intent: IntentId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::DeclareIntent { .. })
        })?;
        self.session
            .context
            .encode_intent_declared_reply(self.session.frame.frame_mut(), nonce, intent)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_task_created(mut self, task: TaskId) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::CreateTask { .. })
        })?;
        self.session
            .context
            .encode_task_created_reply(self.session.frame.frame_mut(), nonce, task)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_task_delegated(
        mut self,
        task: TaskId,
        capability: CapabilityId,
        target: AgentId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::DelegateTask { .. })
        })?;
        self.session
            .context
            .encode_task_delegated_reply(
                self.session.frame.frame_mut(),
                nonce,
                task,
                capability,
                target,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
