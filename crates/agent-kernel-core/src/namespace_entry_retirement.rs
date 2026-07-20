//! Capability-authorized retirement of one Namespace Entry record.
//!
//! This no_std Core transaction validates the active caller, Workspace-scoped
//! rollback authority, and Event capacity before returning one stable dense
//! Store slot. Namespace Entry identities remain monotonic.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, NamespaceEntryId,
    NamespaceEntryRecord, NamespaceEntryRetirement, Operation,
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
    pub fn retire_namespace_entry(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: NamespaceEntryId,
    ) -> Result<NamespaceEntryRetirement, KernelError> {
        self.retire_namespace_entry_transaction(actor, authority, target, None)
    }

    pub fn compare_and_retire_namespace_entry(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: NamespaceEntryId,
        expected_revision: u64,
    ) -> Result<NamespaceEntryRetirement, KernelError> {
        self.retire_namespace_entry_transaction(actor, authority, target, Some(expected_revision))
    }

    fn retire_namespace_entry_transaction(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        target: NamespaceEntryId,
        expected_revision: Option<u64>,
    ) -> Result<NamespaceEntryRetirement, KernelError> {
        self.ensure_agent_active(actor)?;
        let index = self
            .namespace_entries()
            .iter()
            .position(|record| record.id == target)
            .ok_or(KernelError::NamespaceEntryNotFound)?;
        let record = self.namespace_entries[index];
        self.ensure_namespace_resource(record.namespace)?;
        self.ensure_authorized(actor, authority, record.namespace, Operation::Rollback)?;
        if expected_revision.is_some_and(|expected| expected != record.revision) {
            return Err(KernelError::NamespaceRevisionMismatch);
        }
        self.ensure_event_slots(1)?;

        let previous = self.namespace_entries;
        let remaining = self.namespace_entry_len - 1;
        self.namespace_entries[index..remaining]
            .copy_from_slice(&previous[index + 1..self.namespace_entry_len]);
        self.namespace_entries[remaining] = NamespaceEntryRecord::empty();
        self.namespace_entry_len = remaining;
        self.record(namespace_entry_retirement_event(record, actor, authority))?;

        Ok(NamespaceEntryRetirement::new(record, actor, authority))
    }
}

fn namespace_entry_retirement_event(
    record: NamespaceEntryRecord,
    actor: AgentId,
    authority: CapabilityId,
) -> Event {
    let mut event = Event::empty();
    event.agent = actor;
    event.kind = EventKind::NamespaceEntryRetired;
    event.resource = Some(record.namespace);
    event.capability = Some(authority);
    event.namespace_entry = Some(record.id);
    event.namespace_key = Some(record.key);
    event.namespace_object = Some(record.object);
    event.operation = Some(Operation::Rollback);
    event.target_agent = Some(record.owner);
    event
}
