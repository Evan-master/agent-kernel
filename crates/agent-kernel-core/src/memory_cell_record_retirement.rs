//! Supervisor-authorized retirement of one terminal MemoryCell record.
//!
//! This no_std Core transaction validates the backing Resource, active
//! ancestor cleanup authority, Core references, and Event capacity before
//! returning one dense Store slot. MemoryCell IDs remain monotonic.

use crate::{
    AgentEntryKind, AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, MemoryCellId,
    MemoryCellRecord, MemoryCellRecordRetirement, Operation, ResourceKind, ResourceStatus,
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
    pub fn retire_memory_cell_record(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: MemoryCellId,
    ) -> Result<MemoryCellRecordRetirement, KernelError> {
        let entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let index = self
            .memory_cells()
            .iter()
            .position(|record| record.id == target)
            .ok_or(KernelError::MemoryCellNotFound)?;
        let record = self.memory_cells[index];
        let resource = self
            .resources()
            .iter()
            .find(|resource| resource.id == record.resource)
            .copied()
            .ok_or(KernelError::ResourceNotFound)?;
        if resource.kind != ResourceKind::Memory {
            return Err(KernelError::ResourceKindMismatch);
        }
        if resource.status != ResourceStatus::Retired {
            return Err(KernelError::MemoryCellRecordRetirementNotReady);
        }
        self.ensure_cleanup_authorized(actor, authority, record.resource)?;
        self.ensure_memory_cell_record_unreferenced(target)?;
        self.ensure_event_slots(1)?;

        let previous = self.memory_cells;
        let remaining = self.memory_cell_len - 1;
        self.memory_cells[index..remaining]
            .copy_from_slice(&previous[index + 1..self.memory_cell_len]);
        self.memory_cells[remaining] = MemoryCellRecord::empty();
        self.memory_cell_len = remaining;
        self.record(memory_cell_record_retirement_event(
            record, actor, authority,
        ))?;

        Ok(MemoryCellRecordRetirement::new(record, actor, authority))
    }
}

fn memory_cell_record_retirement_event(
    record: MemoryCellRecord,
    actor: AgentId,
    authority: CapabilityId,
) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::MemoryCellRecordRetired;
    event.resource = Some(record.resource);
    event.capability = Some(authority);
    event.memory_cell = Some(record.id);
    event.operation = Some(Operation::Rollback);
    event.target_agent = Some(record.last_writer);
    event
}
