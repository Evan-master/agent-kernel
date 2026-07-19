//! Authorized two-phase Event archive commit.
//!
//! This no_std Core module prepares immutable prefix proposals, validates a
//! launched Supervisor with root Rollback authority, atomically releases live
//! Event slots, and retains the latest cryptographic checkpoint chain head.

use crate::{
    AgentEntryKind, AgentId, CapabilityId, Event, EventArchiveCheckpoint, EventArchiveProposal,
    KernelCore, KernelError, Operation, ResourceStatus,
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
        actor: AgentId,
        authority: CapabilityId,
        proposal: EventArchiveProposal,
    ) -> Result<EventArchiveCheckpoint, KernelError> {
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

        let count = proposal.count();
        let remaining = self.event_len - count;
        self.events.copy_within(count..self.event_len, 0);
        for slot in &mut self.events[remaining..self.event_len] {
            *slot = Event::empty();
        }
        self.event_len = remaining;
        let checkpoint = EventArchiveCheckpoint::new(proposal, actor, authority, root.id);
        self.event_archive_checkpoint = Some(checkpoint);
        Ok(checkpoint)
    }

    pub const fn event_archive_checkpoint(&self) -> Option<EventArchiveCheckpoint> {
        self.event_archive_checkpoint
    }
}
