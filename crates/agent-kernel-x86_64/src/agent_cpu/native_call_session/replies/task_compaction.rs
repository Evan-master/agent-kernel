//! Task compaction acknowledgement for an authenticated Supervisor call.

use agent_kernel_core::TaskCompaction;
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_task_compaction(
        mut self,
        receipt: TaskCompaction,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::CompactTasks { through, .. }
                    if through == receipt.through()
            )
        })?;
        self.session
            .context
            .encode_task_compaction_reply(
                self.session.frame.frame_mut(),
                nonce,
                receipt.first(),
                receipt.through(),
                receipt.count(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
