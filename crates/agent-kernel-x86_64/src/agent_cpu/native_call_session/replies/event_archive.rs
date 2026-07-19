//! Event archive acknowledgement for an authenticated Supervisor call.

use agent_kernel_core::EventArchiveCheckpoint;
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_event_archive(
        mut self,
        checkpoint: EventArchiveCheckpoint,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::ArchiveEvents {
                    through_sequence,
                    ..
                } if through_sequence == checkpoint.through_sequence()
            )
        })?;
        self.session
            .context
            .encode_event_archive_reply(
                self.session.frame.frame_mut(),
                nonce,
                checkpoint.first_sequence(),
                checkpoint.through_sequence(),
                checkpoint.count(),
                checkpoint.digest(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
