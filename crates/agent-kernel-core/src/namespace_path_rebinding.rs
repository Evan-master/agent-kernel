//! Atomic optimistic mutation through a bounded Namespace path.
//!
//! This Core module authenticates every Workspace hop, compares the terminal
//! Entry revision, validates the replacement object, and preflights the full
//! Event transaction before changing Store state. It uses fixed arrays and
//! emits no partial traversal evidence on failure.

use crate::{
    AgentId, EventKind, KernelCore, KernelError, NamespaceEntryRecord, NamespaceObject,
    NamespacePathRebinding, NamespacePathSegment, Operation, ResourceId, NAMESPACE_PATH_MAX_DEPTH,
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
    pub fn compare_and_rebind_namespace_path(
        &mut self,
        actor: AgentId,
        root: ResourceId,
        segments: &[NamespacePathSegment],
        expected_revision: u64,
        replacement: NamespaceObject,
    ) -> Result<NamespacePathRebinding, KernelError> {
        self.ensure_agent_active(actor)?;
        if segments.is_empty() {
            return Err(KernelError::NamespacePathEmpty);
        }
        if segments.len() > NAMESPACE_PATH_MAX_DEPTH {
            return Err(KernelError::NamespacePathTooDeep);
        }

        let mut records = [NamespaceEntryRecord::empty(); NAMESPACE_PATH_MAX_DEPTH];
        let mut workspaces = [ResourceId::new(0); NAMESPACE_PATH_MAX_DEPTH];
        let mut current = root;

        for (index, segment) in segments.iter().copied().enumerate() {
            let terminal = index + 1 == segments.len();
            let operation = if terminal {
                Operation::Act
            } else {
                Operation::Observe
            };
            self.ensure_authorized(actor, segment.authority(), current, operation)?;
            self.ensure_namespace_resource(current)?;
            let record = self.find_namespace_entry_by_key(current, segment.key())?;
            records[index] = record;
            workspaces[index] = current;

            if terminal {
                continue;
            }
            let NamespaceObject::Mount(target) = record.object else {
                return Err(KernelError::NamespaceMountRequired);
            };
            self.ensure_namespace_resource(target)?;
            if workspaces[..=index].contains(&target) {
                return Err(KernelError::NamespaceMountCycle);
            }
            current = target;
        }

        let terminal_index = segments.len() - 1;
        let previous = records[terminal_index];
        if previous.revision != expected_revision {
            return Err(KernelError::NamespaceRevisionMismatch);
        }
        self.ensure_namespace_binding_object(previous.namespace, Some(previous.id), replacement)?;
        let next_revision = previous
            .revision
            .checked_add(1)
            .ok_or(KernelError::NamespaceRevisionExhausted)?;
        self.ensure_event_slots(segments.len())?;

        for index in 0..terminal_index {
            let segment = segments[index];
            let record = records[index];
            self.record_namespace_event(
                EventKind::NamespaceEntryResolved,
                actor,
                segment.authority(),
                workspaces[index],
                record.id,
                segment.key(),
                record.object,
                Operation::Observe,
            )?;
        }

        let resulting = {
            let record = self.find_namespace_entry_mut(previous.id)?;
            record.object = replacement;
            record.revision = next_revision;
            *record
        };
        self.record_namespace_event(
            EventKind::NamespaceEntryRebound,
            actor,
            segments[terminal_index].authority(),
            resulting.namespace,
            resulting.id,
            resulting.key,
            resulting.object,
            Operation::Act,
        )?;

        Ok(NamespacePathRebinding::new(
            root,
            previous,
            resulting,
            segments.len() as u8,
        ))
    }
}
