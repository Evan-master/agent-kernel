//! Event archive acknowledgement for an authenticated Supervisor call.

use agent_kernel_core::{EventArchiveCheckpoint, EventArchiveProposal};
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

    pub(crate) fn acknowledge_event_archive_snapshot(
        mut self,
        proposal: EventArchiveProposal,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::ArchiveEvents {
                    through_sequence,
                    ..
                } if through_sequence == proposal.through_sequence()
            )
        })?;
        self.session
            .context
            .encode_event_archive_reply(
                self.session.frame.frame_mut(),
                nonce,
                proposal.first_sequence(),
                proposal.through_sequence(),
                proposal.count(),
                proposal.digest(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
