//! Authenticated retirement of a terminal Fault prefix.
//!
//! This no_std Core module validates Supervisor identity, Task and Message
//! references, shared Resource cleanup authority, and aggregate Event capacity
//! before returning dense Fault Store slots. Fault IDs remain monotonic.

use crate::{
    AgentEntryKind, AgentId, CapabilityId, Event, EventKind, FaultCompaction, FaultId, FaultRecord,
    KernelCore, KernelError, MessageStatus, Operation, TaskStatus,
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
    pub fn compact_fault_prefix(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        through: FaultId,
    ) -> Result<FaultCompaction, KernelError> {
        let actor_entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if actor_entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let through_index = self
            .faults()
            .iter()
            .position(|record| record.id == through)
            .ok_or(KernelError::FaultNotFound)?;
        let count = through_index + 1;
        let selected = &self.faults()[..count];
        if self.tasks().iter().any(|task| {
            task.status == TaskStatus::Faulted
                && task
                    .last_fault
                    .is_some_and(|fault| selected.iter().any(|record| record.id == fault))
        }) {
            return Err(KernelError::FaultCompactionNotReady);
        }
        if self.messages().iter().any(|message| {
            message.status != MessageStatus::Acknowledged
                && message
                    .payload
                    .fault
                    .is_some_and(|fault| selected.iter().any(|record| record.id == fault))
        }) {
            return Err(KernelError::FaultCompactionReferenced);
        }
        for record in selected.iter().copied() {
            self.ensure_cleanup_authorized(actor, authority, record.resource)?;
        }
        self.ensure_event_slots(count)?;

        let previous = self.faults;
        for task in self.tasks[..self.task_len].iter_mut() {
            if task
                .last_fault
                .is_some_and(|fault| previous[..count].iter().any(|record| record.id == fault))
            {
                task.last_fault = None;
            }
        }
        let remaining = self.fault_len - count;
        self.faults[..remaining].copy_from_slice(&previous[count..self.fault_len]);
        for index in remaining..self.fault_len {
            self.faults[index] = FaultRecord::empty();
        }
        self.fault_len = remaining;
        for record in previous[..count].iter().copied() {
            self.record(fault_compaction_event(record, actor, authority))?;
        }

        Ok(FaultCompaction::new(previous[0].id, through, count))
    }
}

fn fault_compaction_event(record: FaultRecord, actor: AgentId, authority: CapabilityId) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::FaultCompacted;
    event.resource = Some(record.resource);
    event.capability = Some(authority);
    event.operation = Some(Operation::Rollback);
    event.task = Some(record.task);
    event.fault = Some(record.id);
    event.fault_kind = Some(record.kind);
    event.fault_detail = Some(record.detail);
    event.target_agent = Some(record.agent);
    event
}
