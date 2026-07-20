//! Optimistic Namespace generation updates.
//!
//! This module belongs to `agent-kernel-core`. It owns revision-checked rebind
//! transactions and the shared force-rebind implementation. Authorization,
//! object validation, Event capacity, and revision advancement all complete
//! before a caller can observe a resulting record.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, NamespaceEntryId,
    NamespaceEntryRecord, NamespaceObject, Operation,
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
    pub fn compare_and_rebind_namespace_entry(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        entry: NamespaceEntryId,
        expected_revision: u64,
        replacement: NamespaceObject,
    ) -> Result<NamespaceEntryRecord, KernelError> {
        self.rebind_namespace_entry_transaction(
            actor,
            authority,
            entry,
            Some(expected_revision),
            replacement,
        )
        .map(|(record, _)| record)
    }

    pub(crate) fn rebind_namespace_entry_transaction(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        entry: NamespaceEntryId,
        expected_revision: Option<u64>,
        replacement: NamespaceObject,
    ) -> Result<(NamespaceEntryRecord, Event), KernelError> {
        self.ensure_agent_active(actor)?;
        let current = self.find_namespace_entry(entry)?;
        self.ensure_authorized(actor, authority, current.namespace, Operation::Act)?;
        if expected_revision.is_some_and(|expected| expected != current.revision) {
            return Err(KernelError::NamespaceRevisionMismatch);
        }
        self.ensure_namespace_object_exists(replacement)?;
        let next_revision = current
            .revision
            .checked_add(1)
            .ok_or(KernelError::NamespaceRevisionExhausted)?;
        self.ensure_event_slots(1)?;

        let record = self.find_namespace_entry_mut(entry)?;
        record.object = replacement;
        record.revision = next_revision;
        let resulting = *record;
        let event = self.record_namespace_event(
            EventKind::NamespaceEntryRebound,
            actor,
            authority,
            resulting.namespace,
            entry,
            resulting.key,
            replacement,
            Operation::Act,
        )?;
        Ok((resulting, event))
    }
}
