//! Fixed-capacity native object namespace store.
//!
//! This module owns deterministic bind, resolve, and rebind behavior for
//! `agent-kernel-core`. It owns state changes and audit events; lookup and
//! validation helpers live in `namespace_lookup`.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, NamespaceEntryId,
    NamespaceEntryRecord, NamespaceKey, NamespaceObject, Operation, OperationSet, ResourceId,
    VerificationRequirement,
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
    >
{
    pub fn bind_namespace_entry(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        namespace: ResourceId,
        key: NamespaceKey,
        object: NamespaceObject,
    ) -> Result<NamespaceEntryId, KernelError> {
        self.ensure_authorized(agent, capability, namespace, Operation::Act)?;
        self.ensure_namespace_resource(namespace)?;
        if self.find_namespace_entry_by_key(namespace, key).is_ok() {
            return Err(KernelError::NamespaceEntryAlreadyExists);
        }
        self.ensure_namespace_object_exists(object)?;
        if self.namespace_entry_len >= NAMESPACE_ENTRIES {
            return Err(KernelError::NamespaceEntryStoreFull);
        }
        self.ensure_event_slots(1)?;

        let entry = NamespaceEntryId::new(self.next_namespace_entry);
        self.next_namespace_entry += 1;
        self.namespace_entries[self.namespace_entry_len] = NamespaceEntryRecord {
            id: entry,
            owner: agent,
            namespace,
            capability,
            key,
            object,
            revision: 1,
        };
        self.namespace_entry_len += 1;
        self.record_namespace_event(
            EventKind::NamespaceEntryBound,
            agent,
            capability,
            namespace,
            entry,
            key,
            object,
            Operation::Act,
        )?;
        Ok(entry)
    }

    pub fn resolve_namespace_entry(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        namespace: ResourceId,
        key: NamespaceKey,
    ) -> Result<NamespaceObject, KernelError> {
        self.ensure_authorized(agent, capability, namespace, Operation::Observe)?;
        self.ensure_namespace_resource(namespace)?;
        let record = self.find_namespace_entry_by_key(namespace, key)?;
        self.ensure_event_slots(1)?;

        self.record_namespace_event(
            EventKind::NamespaceEntryResolved,
            agent,
            capability,
            namespace,
            record.id,
            key,
            record.object,
            Operation::Observe,
        )?;
        Ok(record.object)
    }

    pub fn rebind_namespace_entry(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        entry: NamespaceEntryId,
        object: NamespaceObject,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let record = self.find_namespace_entry(entry)?;
        self.ensure_authorized(agent, capability, record.namespace, Operation::Act)?;
        self.ensure_namespace_object_exists(object)?;
        self.ensure_event_slots(1)?;

        let namespace_entry = self.find_namespace_entry_mut(entry)?;
        namespace_entry.object = object;
        namespace_entry.revision += 1;
        self.record_namespace_event(
            EventKind::NamespaceEntryRebound,
            agent,
            capability,
            record.namespace,
            entry,
            record.key,
            object,
            Operation::Act,
        )
    }

    fn record_namespace_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: CapabilityId,
        namespace: ResourceId,
        entry: NamespaceEntryId,
        key: NamespaceKey,
        object: NamespaceObject,
        operation: Operation,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind,
            resource: Some(namespace),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: Some(entry),
            namespace_key: Some(key),
            namespace_object: Some(object),
            operation: Some(operation),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            signal: None,
            target_agent: None,
            driver_binding: None,
            device_event: None,
            device_event_kind: None,
            device_event_payload: None,
            driver_command: None,
            driver_command_kind: None,
            driver_command_payload: None,
            driver_command_result: None,
            driver_invocation: None,
            driver_invocation_ticks: None,
            driver_invocation_quantum: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
