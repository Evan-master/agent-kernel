//! Read-only authorization boundary for one durable Event archive commit.
//!
//! Core validates the launched Supervisor, root Rollback authority, storage
//! Checkpoint authority, and exact current proposal before machine code may
//! write durable media. The returned record is immutable and emits no Event.

use crate::{
    AgentEntryKind, AgentId, CapabilityId, EventArchiveProposal, KernelCore, KernelError,
    Operation, ResourceId, ResourceStatus,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchivePreflight {
    actor: AgentId,
    archive_authority: CapabilityId,
    storage_authority: CapabilityId,
    root: ResourceId,
    storage: ResourceId,
    proposal: EventArchiveProposal,
}

impl DurableArchivePreflight {
    pub const fn actor(self) -> AgentId {
        self.actor
    }

    pub const fn archive_authority(self) -> CapabilityId {
        self.archive_authority
    }

    pub const fn storage_authority(self) -> CapabilityId {
        self.storage_authority
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }

    pub const fn storage(self) -> ResourceId {
        self.storage
    }

    pub const fn proposal(self) -> EventArchiveProposal {
        self.proposal
    }
}

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
    pub fn preflight_durable_event_archive(
        &self,
        actor: AgentId,
        archive_authority: CapabilityId,
        storage_authority: CapabilityId,
        storage: ResourceId,
        proposal: EventArchiveProposal,
    ) -> Result<DurableArchivePreflight, KernelError> {
        let entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let capability = self.find_capability(archive_authority)?;
        let root = self.find_resource(capability.resource)?;
        if root.parent.is_some() {
            return Err(KernelError::EventArchiveAuthorityScopeMismatch);
        }
        if root.status != ResourceStatus::Active {
            return Err(KernelError::ResourceRetired);
        }
        self.ensure_authorized(actor, archive_authority, root.id, Operation::Rollback)?;

        let current = self
            .prepare_event_archive(proposal.through_sequence())
            .map_err(|_| KernelError::EventArchiveProposalMismatch)?;
        if current != proposal {
            return Err(KernelError::EventArchiveProposalMismatch);
        }

        let storage_record = self.find_resource(storage)?;
        if storage_record.status != ResourceStatus::Active {
            return Err(KernelError::ResourceRetired);
        }
        self.ensure_authorized(actor, storage_authority, storage, Operation::Checkpoint)?;

        Ok(DurableArchivePreflight {
            actor,
            archive_authority,
            storage_authority,
            root: root.id,
            storage,
            proposal,
        })
    }
}
