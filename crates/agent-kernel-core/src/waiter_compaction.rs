//! Authenticated retirement of an inactive Waiter prefix.
//!
//! This no_std Core module validates Supervisor identity, shared Resource
//! cleanup authority, terminal waiter state, and aggregate Event capacity
//! before returning dense Store slots. Waiter IDs remain monotonic.

use crate::{
    AgentEntryKind, AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, Operation,
    WaiterCompaction, WaiterId, WaiterRecord,
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
    pub fn compact_waiter_prefix(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        through: WaiterId,
    ) -> Result<WaiterCompaction, KernelError> {
        let actor_entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if actor_entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let through_index = self
            .waiters()
            .iter()
            .position(|record| record.id == through)
            .ok_or(KernelError::WaiterNotFound)?;
        let count = through_index + 1;
        for record in self.waiters()[..count].iter().copied() {
            if record.active {
                return Err(KernelError::WaiterCompactionNotReady);
            }
            self.ensure_cleanup_authorized(actor, authority, record.resource)?;
        }
        self.ensure_event_slots(count)?;

        let previous = self.waiters;
        let remaining = self.waiter_len - count;
        self.waiters[..remaining].copy_from_slice(&previous[count..self.waiter_len]);
        for index in remaining..self.waiter_len {
            self.waiters[index] = WaiterRecord::empty();
        }
        self.waiter_len = remaining;
        for record in previous[..count].iter().copied() {
            self.record(waiter_compaction_event(record, actor, authority))?;
        }

        Ok(WaiterCompaction::new(previous[0].id, through, count))
    }
}

fn waiter_compaction_event(record: WaiterRecord, actor: AgentId, authority: CapabilityId) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::WaiterCompacted;
    event.resource = Some(record.resource);
    event.capability = Some(authority);
    event.operation = Some(Operation::Rollback);
    event.task = Some(record.task);
    event.waiter = Some(record.id);
    event.waiter_kind = Some(record.kind);
    event.signal = Some(record.signal);
    event.target_agent = Some(record.agent);
    event
}
