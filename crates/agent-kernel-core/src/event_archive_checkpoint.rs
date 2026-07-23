//! Authorized two-phase Event archive commit.
//!
//! This no_std Core module prepares immutable prefix proposals, validates a
//! launched Supervisor with root Rollback authority, atomically releases live
//! Event slots, and retains the latest cryptographic checkpoint chain head.

use crate::{
    AgentEntryKind, AgentId, CapabilityId, DurableArchiveCommitProof, DurableArchiveReceipt,
    DurableArchiveRecoveryVerificationRequest, DurableArchiveRecoveryVerifier,
    DurableArchiveVerificationRequest, DurableArchiveVerifier, DurableRecoveredHead, Event,
    EventArchiveCheckpoint, EventArchiveProposal, KernelCore, KernelError, Operation, ResourceId,
    ResourceStatus,
};

impl<
        const AGENTS: usize,
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
        const MESSAGES: usize,
        const MEMORY_CELLS: usize,
        const NAMESPACE_ENTRIES: usize,
        const FAULTS: usize,
        const FAULT_HANDLERS: usize,
        const FAULT_POLICIES: usize,
        const WAITERS: usize,
        const AGENT_IMAGES: usize,
        const DRIVER_BINDINGS: usize,
        const DEVICE_EVENTS: usize,
        const DRIVER_COMMANDS: usize,
        const DRIVER_INVOCATIONS: usize,
        const RUNTIME_ADMISSIONS: usize,
    >
    KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        MESSAGES,
        MEMORY_CELLS,
        NAMESPACE_ENTRIES,
        FAULTS,
        FAULT_HANDLERS,
        FAULT_POLICIES,
        WAITERS,
        AGENT_IMAGES,
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
        RUNTIME_ADMISSIONS,
    >
{
    pub fn prepare_event_archive(
        &self,
        through_sequence: u64,
    ) -> Result<EventArchiveProposal, KernelError> {
        let through_index = self
            .events()
            .iter()
            .position(|event| event.sequence == through_sequence)
            .ok_or(KernelError::EventArchiveSequenceNotFound)?;
        EventArchiveProposal::from_segment(
            self.event_archive_checkpoint,
            &self.events()[..=through_index],
        )
        .ok_or(KernelError::EventArchiveProposalMismatch)
    }

    pub fn commit_event_archive(
        &mut self,
        _actor: AgentId,
        _authority: CapabilityId,
        _proposal: EventArchiveProposal,
    ) -> Result<EventArchiveCheckpoint, KernelError> {
        Err(KernelError::EventArchiveDurabilityRequired)
    }

    pub fn commit_durable_event_archive<V: DurableArchiveVerifier>(
        &mut self,
        actor: AgentId,
        archive_authority: CapabilityId,
        storage_authority: CapabilityId,
        proposal: EventArchiveProposal,
        receipt: DurableArchiveReceipt,
        verifier: &mut V,
    ) -> Result<EventArchiveCheckpoint, KernelError> {
        if self.durable_archive_receipt == Some(receipt) {
            return Err(KernelError::EventArchiveReceiptReplay);
        }
        let root = self.validate_event_archive_commit(actor, archive_authority, proposal)?;
        if !receipt.matches_proposal_values(proposal) {
            return Err(KernelError::EventArchiveReceiptMismatch);
        }
        let storage = self.find_resource(receipt.storage())?;
        if storage.status != ResourceStatus::Active {
            return Err(KernelError::ResourceRetired);
        }
        self.ensure_authorized(
            actor,
            storage_authority,
            receipt.storage(),
            Operation::Checkpoint,
        )?;
        let request = DurableArchiveVerificationRequest::new(
            proposal,
            actor,
            archive_authority,
            storage_authority,
            root,
            receipt,
        );
        verifier
            .verify(request)
            .map_err(|_| KernelError::EventArchiveVerificationFailed)?;
        let proof = DurableArchiveCommitProof::new(request);
        Ok(self.apply_durable_archive_commit(proof))
    }

    pub fn recover_durable_event_archive<V: DurableArchiveRecoveryVerifier>(
        &mut self,
        head: DurableRecoveredHead,
        verifier: &mut V,
    ) -> Result<EventArchiveCheckpoint, KernelError> {
        if self.event_len != 0
            || self.event_archive_checkpoint.is_some()
            || self.durable_archive_receipt.is_some()
            || self.next_sequence != 1
        {
            return Err(KernelError::EventArchiveRecoveryStateNotVirgin);
        }
        let next_sequence = head
            .through_sequence()
            .checked_add(1)
            .ok_or(KernelError::EventArchiveRecoverySequenceExhausted)?;
        let request = DurableArchiveRecoveryVerificationRequest::new(head);
        verifier
            .verify(request)
            .map_err(|_| KernelError::EventArchiveRecoveryVerificationFailed)?;

        let checkpoint = EventArchiveCheckpoint::from_recovered_head(head);
        self.event_archive_checkpoint = Some(checkpoint);
        self.durable_archive_receipt = Some(head.receipt());
        self.next_sequence = next_sequence;
        Ok(checkpoint)
    }

    fn validate_event_archive_commit(
        &self,
        actor: AgentId,
        authority: CapabilityId,
        proposal: EventArchiveProposal,
    ) -> Result<ResourceId, KernelError> {
        let entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let capability = self.find_capability(authority)?;
        let root = self.find_resource(capability.resource)?;
        if root.parent.is_some() {
            return Err(KernelError::EventArchiveAuthorityScopeMismatch);
        }
        if root.status != ResourceStatus::Active {
            return Err(KernelError::ResourceRetired);
        }
        self.ensure_authorized(actor, authority, root.id, Operation::Rollback)?;

        let current = self
            .prepare_event_archive(proposal.through_sequence())
            .map_err(|_| KernelError::EventArchiveProposalMismatch)?;
        if current != proposal {
            return Err(KernelError::EventArchiveProposalMismatch);
        }

        Ok(root.id)
    }

    fn apply_event_archive_commit(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        root: ResourceId,
        proposal: EventArchiveProposal,
        receipt: DurableArchiveReceipt,
    ) -> EventArchiveCheckpoint {
        let count = proposal.count();
        let remaining = self.event_len - count;
        self.events.copy_within(count..self.event_len, 0);
        for slot in &mut self.events[remaining..self.event_len] {
            *slot = Event::empty();
        }
        self.event_len = remaining;
        let checkpoint = EventArchiveCheckpoint::new(proposal, actor, authority, root);
        self.event_archive_checkpoint = Some(checkpoint);
        self.durable_archive_receipt = Some(receipt);
        checkpoint
    }

    fn apply_durable_archive_commit(
        &mut self,
        proof: DurableArchiveCommitProof,
    ) -> EventArchiveCheckpoint {
        let request = proof.request();
        self.apply_event_archive_commit(
            request.actor(),
            request.archive_authority(),
            request.root(),
            request.proposal(),
            request.receipt(),
        )
    }

    pub const fn event_archive_checkpoint(&self) -> Option<EventArchiveCheckpoint> {
        self.event_archive_checkpoint
    }

    pub const fn durable_archive_receipt(&self) -> Option<DurableArchiveReceipt> {
        self.durable_archive_receipt
    }
}
