//! Supervisor-authorized retirement of one terminal Resource record.
//!
//! This no_std Core module validates lifecycle, ancestor cleanup authority,
//! every non-Event reference, and Event capacity before returning one dense
//! Resource Store slot. The monotonic Resource allocator remains untouched.

use crate::{
    AgentEntryKind, AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, Operation,
    Resource, ResourceId, ResourceRecordRetirement, ResourceStatus,
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
    pub fn retire_resource_record(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: ResourceId,
    ) -> Result<ResourceRecordRetirement, KernelError> {
        let entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let index = self
            .resources()
            .iter()
            .position(|record| record.id == target)
            .ok_or(KernelError::ResourceNotFound)?;
        let record = self.resources[index];
        if record.status != ResourceStatus::Retired {
            return Err(KernelError::ResourceRecordRetirementNotReady);
        }
        self.ensure_cleanup_authorized(actor, authority, target)?;
        self.ensure_resource_record_unreferenced(target)?;
        self.ensure_event_slots(1)?;

        let previous = self.resources;
        let remaining = self.resource_len - 1;
        self.resources[index..remaining].copy_from_slice(&previous[index + 1..self.resource_len]);
        self.resources[remaining] = Resource::empty();
        self.resource_len = remaining;
        self.record(resource_record_retirement_event(record, actor, authority))?;

        Ok(ResourceRecordRetirement::new(record, actor, authority))
    }
}

fn resource_record_retirement_event(
    record: Resource,
    actor: AgentId,
    authority: CapabilityId,
) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::ResourceRecordRetired;
    event.resource = Some(record.id);
    event.capability = Some(authority);
    event.operation = Some(Operation::Rollback);
    event.target_agent = record.owner;
    event
}
