//! Namespace lookup and validation helpers.
//!
//! This module belongs to `agent-kernel-core`. It keeps object-reference
//! validation and fixed-capacity namespace lookup separate from the mutating
//! bind, resolve, and rebind state machine.

use crate::{
    KernelCore, KernelError, NamespaceEntryId, NamespaceEntryRecord, NamespaceKey, NamespaceObject,
    ResourceId, ResourceKind,
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
    pub fn namespace_entries(&self) -> &[NamespaceEntryRecord] {
        &self.namespace_entries[..self.namespace_entry_len]
    }

    pub const fn namespace_entry_capacity(&self) -> usize {
        NAMESPACE_ENTRIES
    }

    pub(crate) fn ensure_namespace_resource(
        &self,
        namespace: ResourceId,
    ) -> Result<(), KernelError> {
        if self.find_resource(namespace)?.kind == ResourceKind::Workspace {
            Ok(())
        } else {
            Err(KernelError::ResourceKindMismatch)
        }
    }

    pub(crate) fn ensure_namespace_object_exists(
        &self,
        object: NamespaceObject,
    ) -> Result<(), KernelError> {
        match object {
            NamespaceObject::Agent(agent) => self.find_agent(agent).map(|_| ()),
            NamespaceObject::Resource(resource) => self.find_resource(resource).map(|_| ()),
            NamespaceObject::Task(task) => self.find_task(task).map(|_| ()),
            NamespaceObject::Message(message) => self.find_message(message).map(|_| ()),
            NamespaceObject::MemoryCell(cell) => self.find_memory_cell(cell).map(|_| ()),
            NamespaceObject::Mount(target) => self.ensure_namespace_resource(target),
        }
    }

    pub(crate) fn ensure_namespace_binding_object(
        &self,
        source: ResourceId,
        ignored: Option<NamespaceEntryId>,
        object: NamespaceObject,
    ) -> Result<(), KernelError> {
        self.ensure_namespace_object_exists(object)?;
        let NamespaceObject::Mount(target) = object else {
            return Ok(());
        };
        self.ensure_namespace_mount_acyclic(source, target, ignored)
    }

    fn ensure_namespace_mount_acyclic(
        &self,
        source: ResourceId,
        target: ResourceId,
        ignored: Option<NamespaceEntryId>,
    ) -> Result<(), KernelError> {
        if source == target {
            return Err(KernelError::NamespaceMountCycle);
        }

        let mut discovered = [ResourceId::new(0); NAMESPACE_ENTRIES];
        let mut discovered_len = 0;
        let mut cursor = 0;
        let mut current = target;

        loop {
            for record in self.namespace_entries() {
                if Some(record.id) == ignored || record.namespace != current {
                    continue;
                }
                let NamespaceObject::Mount(next) = record.object else {
                    continue;
                };
                if next == source {
                    return Err(KernelError::NamespaceMountCycle);
                }
                if next == target || discovered[..discovered_len].contains(&next) {
                    continue;
                }
                if discovered_len >= NAMESPACE_ENTRIES {
                    return Err(KernelError::NamespaceMountCycle);
                }
                discovered[discovered_len] = next;
                discovered_len += 1;
            }

            if cursor >= discovered_len {
                return Ok(());
            }
            current = discovered[cursor];
            cursor += 1;
        }
    }

    pub(crate) fn find_namespace_entry(
        &self,
        id: NamespaceEntryId,
    ) -> Result<NamespaceEntryRecord, KernelError> {
        self.namespace_entries()
            .iter()
            .find(|entry| entry.id == id)
            .copied()
            .ok_or(KernelError::NamespaceEntryNotFound)
    }

    pub(crate) fn find_namespace_entry_by_key(
        &self,
        namespace: ResourceId,
        key: NamespaceKey,
    ) -> Result<NamespaceEntryRecord, KernelError> {
        self.namespace_entries()
            .iter()
            .find(|entry| entry.namespace == namespace && entry.key == key)
            .copied()
            .ok_or(KernelError::NamespaceEntryNotFound)
    }

    pub(crate) fn find_namespace_entry_mut(
        &mut self,
        id: NamespaceEntryId,
    ) -> Result<&mut NamespaceEntryRecord, KernelError> {
        self.namespace_entries[..self.namespace_entry_len]
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(KernelError::NamespaceEntryNotFound)
    }
}
