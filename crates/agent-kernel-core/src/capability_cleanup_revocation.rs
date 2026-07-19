//! Supervisor-authorized revocation for terminal Resource cleanup.
//!
//! This Core module lets a launched Supervisor use active ancestor Rollback
//! authority to revoke one Capability on a retired Resource. It performs no
//! compaction; the existing Capability compactor remains the only slot owner.

use crate::{
    AgentEntryKind, AgentId, Capability, CapabilityId, Event, EventKind, KernelCore, KernelError,
    Operation, ResourceStatus,
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
    pub fn revoke_capability_for_cleanup(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: CapabilityId,
    ) -> Result<Event, KernelError> {
        let entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let target_record = self.find_capability(target)?;
        if target_record.revoked {
            return Err(KernelError::CapabilityRevoked);
        }
        let resource = self
            .resources()
            .iter()
            .find(|record| record.id == target_record.resource)
            .copied()
            .ok_or(KernelError::ResourceNotFound)?;
        if resource.status != ResourceStatus::Retired {
            return Err(KernelError::CapabilityCleanupNotReady);
        }
        self.ensure_cleanup_authorized(actor, authority, resource.id)?;
        self.ensure_event_slots(1)?;

        self.find_capability_mut(target)?.revoked = true;
        self.record(capability_cleanup_event(target_record, actor, authority))
    }
}

fn capability_cleanup_event(target: Capability, actor: AgentId, authority: CapabilityId) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::CapabilityRevoked;
    event.resource = Some(target.resource);
    event.capability = Some(target.id);
    event.source_capability = Some(authority);
    event.operation = Some(Operation::Rollback);
    event.operations = target.operations;
    event.task = target.task;
    event.target_agent = Some(target.agent);
    event
}
