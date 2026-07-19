//! Core-owned live-reference preflight for MemoryCell record retirement.
//!
//! This no_std Core child rejects Namespace objects that still resolve to the
//! target. Historical Events stay outside the live-reference set because the
//! monotonic MemoryCell allocator prevents identity reuse.

use crate::{KernelCore, KernelError, MemoryCellId, NamespaceObject};

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
    pub(crate) fn ensure_memory_cell_record_unreferenced(
        &self,
        target: MemoryCellId,
    ) -> Result<(), KernelError> {
        if self.namespace_entries[..self.namespace_entry_len]
            .iter()
            .any(|entry| entry.object == NamespaceObject::MemoryCell(target))
        {
            Err(KernelError::MemoryCellRecordRetirementReferenced)
        } else {
            Ok(())
        }
    }
}
