//! Acknowledgements for authenticated two-stage durable archive calls.

use agent_kernel_core::EventArchiveCheckpoint;
use agent_kernel_x86_64::{agent_call::AgentCallRequest, ata::NativeDurableArchivePreparation};

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_durable_archive_prepared(
        mut self,
        preparation: NativeDurableArchivePreparation,
    ) -> Option<ResumableAgentCpu> {
        let preflight = preparation.preflight();
        let proposal = preflight.proposal();
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::PrepareDurableArchive {
                    archive_authority,
                    storage_authority,
                    through_sequence,
                    generation,
                    ..
                } if archive_authority == preflight.archive_authority()
                    && storage_authority == preflight.storage_authority()
                    && through_sequence == proposal.through_sequence()
                    && generation == preparation.call_data_generation()
            )
        })?;
        self.session
            .context
            .encode_durable_archive_prepare_reply(
                self.session.frame.frame_mut(),
                nonce,
                proposal.generation(),
                proposal.first_sequence(),
                proposal.through_sequence(),
                proposal.count(),
                preparation.manifest().signer_policy_generation(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_durable_archive_committed(
        mut self,
        call_data_generation: u64,
        checkpoint: EventArchiveCheckpoint,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::CommitDurableArchiveFromMemory { generation, .. }
                    if generation == call_data_generation
            )
        })?;
        self.session
            .context
            .encode_durable_archive_commit_reply(
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
