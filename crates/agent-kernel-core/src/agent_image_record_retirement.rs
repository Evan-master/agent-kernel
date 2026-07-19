//! Authenticated retirement of terminal Agent Image records.
//!
//! This no_std Core module validates image lifecycle, shared Resource cleanup
//! authority, and every non-Event image reference before returning one dense
//! Store slot. Image IDs remain monotonic because the allocator is untouched.

use crate::{
    AgentId, AgentImageId, AgentImageRecord, AgentImageRecordRetirement, AgentImageStatus,
    CapabilityId, Event, EventKind, KernelCore, KernelError, Operation,
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
    pub fn retire_agent_image_record(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        image: AgentImageId,
    ) -> Result<AgentImageRecordRetirement, KernelError> {
        self.ensure_agent_active(actor)?;
        let index = self
            .agent_images()
            .iter()
            .position(|record| record.id == image)
            .ok_or(KernelError::AgentImageNotFound)?;
        let record = self.agent_images[index];
        if record.status != AgentImageStatus::Retired {
            return Err(KernelError::AgentImageRecordRetirementNotReady);
        }
        self.ensure_cleanup_authorized(actor, authority, record.resource)?;
        self.ensure_agent_image_record_unreferenced(image)?;
        self.ensure_event_slots(1)?;

        let previous = self.agent_images;
        let remaining = self.agent_image_len - 1;
        self.agent_images[index..remaining]
            .copy_from_slice(&previous[index + 1..self.agent_image_len]);
        self.agent_images[remaining] = AgentImageRecord::empty();
        self.agent_image_len = remaining;
        self.record(agent_image_record_retirement_event(
            record, actor, authority,
        ))?;

        Ok(AgentImageRecordRetirement::new(record, actor, authority))
    }

    fn ensure_agent_image_record_unreferenced(
        &self,
        image: AgentImageId,
    ) -> Result<(), KernelError> {
        let referenced = self
            .agent_entries()
            .iter()
            .any(|entry| entry.image == image)
            || self
                .runtime_admissions()
                .iter()
                .any(|admission| admission.image == image);
        if referenced {
            Err(KernelError::AgentImageRecordRetirementReferenced)
        } else {
            Ok(())
        }
    }
}

fn agent_image_record_retirement_event(
    record: AgentImageRecord,
    actor: AgentId,
    authority: CapabilityId,
) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::AgentImageRecordRetired;
    event.resource = Some(record.resource);
    event.capability = Some(authority);
    event.operation = Some(Operation::Rollback);
    event.target_agent = Some(record.owner);
    event.agent_image = Some(record.id);
    event.agent_image_kind = Some(record.kind);
    event.agent_image_digest = Some(record.digest);
    event.agent_image_abi_version = Some(record.abi_version);
    event.agent_image_entry_version = Some(record.entry_version);
    event
}
