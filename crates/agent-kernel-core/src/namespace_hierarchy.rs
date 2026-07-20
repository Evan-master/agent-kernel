//! Bounded hierarchical Namespace resolution.
//!
//! This Core module validates every Workspace authority and mount hop before
//! emitting ordered resolution Events. It uses fixed arrays, performs no path
//! parsing or allocation, and never exposes a partial traversal transcript.

use crate::{
    AgentId, EventKind, KernelCore, KernelError, NamespaceEntryRecord, NamespaceObject,
    NamespacePathResolution, NamespacePathSegment, Operation, ResourceId, NAMESPACE_PATH_MAX_DEPTH,
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
    pub fn resolve_namespace_path(
        &mut self,
        actor: AgentId,
        root: ResourceId,
        segments: &[NamespacePathSegment],
    ) -> Result<NamespacePathResolution, KernelError> {
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
            self.ensure_authorized(actor, segment.authority(), current, Operation::Observe)?;
            self.ensure_namespace_resource(current)?;
            let record = self.find_namespace_entry_by_key(current, segment.key())?;
            records[index] = record;
            workspaces[index] = current;

            if index + 1 == segments.len() {
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

        self.ensure_event_slots(segments.len())?;
        for (index, segment) in segments.iter().copied().enumerate() {
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

        let terminal = records[segments.len() - 1];
        Ok(NamespacePathResolution::new(
            root,
            terminal,
            segments.len() as u8,
        ))
    }
}
