//! Public facade for two-phase Event archive checkpoints.
//!
//! This no_std boundary exposes immutable proposal preparation, authorized
//! commit, and latest-checkpoint inspection while Core retains canonical
//! hashing, authority validation, and dense Event Store mutation.

use agent_kernel_core::{
    AgentId, CapabilityId, DurableArchiveReceipt, DurableArchiveRecoveryVerifier,
    DurableArchiveVerifier, DurableRecoveredHead, EventArchiveCheckpoint, EventArchiveProposal,
    KernelError,
};

use crate::AgentKernel;

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
    AgentKernel<
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
    pub fn sys_prepare_event_archive(
        &self,
        through_sequence: u64,
    ) -> Result<EventArchiveProposal, KernelError> {
        self.core.prepare_event_archive(through_sequence)
    }

    pub fn sys_commit_event_archive(
        &mut self,
        _actor: AgentId,
        _authority: CapabilityId,
        _proposal: EventArchiveProposal,
    ) -> Result<EventArchiveCheckpoint, KernelError> {
        Err(KernelError::EventArchiveDurabilityRequired)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn commit_verified_event_archive<V: DurableArchiveVerifier>(
        &mut self,
        actor: AgentId,
        archive_authority: CapabilityId,
        storage_authority: CapabilityId,
        proposal: EventArchiveProposal,
        receipt: DurableArchiveReceipt,
        verifier: &mut V,
    ) -> Result<EventArchiveCheckpoint, KernelError> {
        self.core.commit_durable_event_archive(
            actor,
            archive_authority,
            storage_authority,
            proposal,
            receipt,
            verifier,
        )
    }

    pub fn recover_verified_event_archive<V: DurableArchiveRecoveryVerifier>(
        &mut self,
        head: DurableRecoveredHead,
        verifier: &mut V,
    ) -> Result<EventArchiveCheckpoint, KernelError> {
        self.core.recover_durable_event_archive(head, verifier)
    }

    pub const fn event_archive_checkpoint(&self) -> Option<EventArchiveCheckpoint> {
        self.core.event_archive_checkpoint()
    }

    pub const fn durable_archive_receipt(&self) -> Option<DurableArchiveReceipt> {
        self.core.durable_archive_receipt()
    }
}
