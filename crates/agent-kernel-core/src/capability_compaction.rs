//! Authenticated retirement of one revoked Capability record.
//!
//! Capability storage is sparse, so compaction clears one slot after checking
//! live references, cleanup authority, and Event capacity. Capability IDs stay
//! monotonic and historical Events keep the retired grant metadata replayable.

use crate::{
    AgentEntryKind, AgentId, Capability, CapabilityCompaction, CapabilityId, Event, EventKind,
    KernelCore, KernelError, MessageStatus, Operation, ResourceId, ResourceStatus,
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
    pub fn compact_capability(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: CapabilityId,
    ) -> Result<CapabilityCompaction, KernelError> {
        let actor_entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if actor_entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let target_record = self.find_capability(target)?;
        if !target_record.revoked {
            return Err(KernelError::CapabilityCompactionNotReady);
        }

        self.ensure_capability_compaction_authority(actor, authority, target_record.resource)?;
        self.ensure_capability_unreferenced(target)?;
        self.ensure_event_slots(1)?;

        let slot = self
            .capabilities
            .iter()
            .position(|record| record.is_some_and(|capability| capability.id == target))
            .ok_or(KernelError::CapabilityNotFound)?;
        self.capabilities[slot] = None;
        self.record(capability_compaction_event(target_record, actor, authority))?;

        Ok(CapabilityCompaction::new(target))
    }

    fn ensure_capability_unreferenced(&self, target: CapabilityId) -> Result<(), KernelError> {
        let referenced = self
            .capabilities
            .iter()
            .flatten()
            .any(|capability| capability.parent == Some(target))
            || self
                .tasks()
                .iter()
                .any(|task| task.delegated_capability == Some(target))
            || self
                .agent_entries()
                .iter()
                .any(|entry| entry.capability == target)
            || self
                .runtime_admissions()
                .iter()
                .any(|admission| admission.authority == target)
            || self.messages().iter().any(|message| {
                message.payload.capability == Some(target)
                    && message.status != MessageStatus::Acknowledged
            });

        if referenced {
            Err(KernelError::CapabilityCompactionReferenced)
        } else {
            Ok(())
        }
    }

    fn ensure_capability_compaction_authority(
        &self,
        actor: AgentId,
        authority: CapabilityId,
        target_resource: ResourceId,
    ) -> Result<(), KernelError> {
        let authority_record = self.find_capability(authority)?;
        self.ensure_authorized(
            actor,
            authority,
            authority_record.resource,
            Operation::Rollback,
        )?;

        let target_record = self
            .resources()
            .iter()
            .find(|resource| resource.id == target_resource)
            .copied()
            .ok_or(KernelError::ResourceNotFound)?;
        if target_record.status == ResourceStatus::Active {
            return if authority_record.resource == target_resource {
                Ok(())
            } else {
                Err(KernelError::ResourceMismatch)
            };
        }

        let mut current = target_record;
        for _ in 0..RESOURCES {
            let Some(parent) = current.parent else {
                return Err(KernelError::ResourceMismatch);
            };
            if parent == authority_record.resource {
                return Ok(());
            }
            current = self
                .resources()
                .iter()
                .find(|resource| resource.id == parent)
                .copied()
                .ok_or(KernelError::ResourceNotFound)?;
        }

        Err(KernelError::ResourceMismatch)
    }
}

fn capability_compaction_event(
    target: Capability,
    actor: AgentId,
    authority: CapabilityId,
) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::CapabilityCompacted;
    event.resource = Some(target.resource);
    event.capability = Some(target.id);
    event.source_capability = Some(authority);
    event.operation = Some(Operation::Rollback);
    event.operations = target.operations;
    event.task = target.task;
    event.target_agent = Some(target.agent);
    event
}
