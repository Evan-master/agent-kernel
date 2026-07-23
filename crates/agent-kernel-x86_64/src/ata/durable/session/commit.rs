//! Signed Event archive orchestration over one initialized ATA session.

use agent_kernel_core::{
    encode_event_archive_payload, AgentId, CapabilityId, DurableArchiveManifest,
    DurableStateDigest, Event, EventArchiveEncodingError, EventArchiveProposal,
};

use crate::{
    ata::{AtaBlockDevice, NativeAtaDurableSession},
    durable_archive_request::DurableArchiveRequest,
    durable_state::{
        commit_durable_archive, DurableArchiveCommitError, DurableStateTrustPolicy,
        VerifiedDurableArchiveCommit,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeAtaDurableCommitError {
    Archive(EventArchiveEncodingError),
    ManifestMismatch,
    Transaction(DurableArchiveCommitError),
}

impl<'a, D: AtaBlockDevice> NativeAtaDurableSession<'a, D> {
    pub fn commit(
        &mut self,
        actor: AgentId,
        archive_authority: CapabilityId,
        proposal: EventArchiveProposal,
        events: &[Event],
        request: DurableArchiveRequest,
    ) -> Result<VerifiedDurableArchiveCommit, NativeAtaDurableCommitError> {
        let payload_length = encode_event_archive_payload(proposal, events, self.payload.as_mut())
            .map_err(NativeAtaDurableCommitError::Archive)?;
        let submitted = request.manifest();
        let expected = DurableArchiveManifest::new(
            proposal,
            actor,
            archive_authority,
            self.config.root(),
            self.config.storage(),
            payload_length as u32,
            DurableStateDigest::from_archive(proposal.digest()),
            submitted.signer_id(),
            submitted.signer_policy_generation(),
            submitted.anchor(),
        )
        .map_err(|_| NativeAtaDurableCommitError::ManifestMismatch)?;
        let signer = self.config.signer();
        if submitted != expected
            || submitted.signer_id() != signer.signer_id
            || submitted.signer_policy_generation() != self.config.policy_generation()
        {
            return Err(NativeAtaDurableCommitError::ManifestMismatch);
        }
        let policy = DurableStateTrustPolicy::new(
            core::slice::from_ref(&signer),
            self.config.policy_generation(),
        );
        commit_durable_archive(
            &mut self.backend,
            policy,
            &self.payload[..payload_length],
            submitted,
            request.signature(),
            self.scratch.as_mut(),
        )
        .map_err(NativeAtaDurableCommitError::Transaction)
    }
}
