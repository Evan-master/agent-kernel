//! Runtime admission acknowledgements for authenticated Supervisor and Worker calls.

use agent_kernel_core::{AgentId, RuntimeAdmissionId, TaskId};
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_runtime_admission_discovery(mut self) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::DiscoverRuntimeAdmission { .. })
        })?;
        self.session
            .context
            .encode_runtime_admission_discovery_reply(self.session.frame.frame_mut(), nonce)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_runtime_admission(
        mut self,
        admission: RuntimeAdmissionId,
        target: AgentId,
        target_task: TaskId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RequestRuntimeAdmission {
                    target: requested_target,
                    target_task: requested_task,
                    ..
                } if requested_target == target && requested_task == target_task
            )
        })?;
        self.session
            .context
            .encode_runtime_admission_reply(
                self.session.frame.frame_mut(),
                nonce,
                admission,
                target,
                target_task,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
