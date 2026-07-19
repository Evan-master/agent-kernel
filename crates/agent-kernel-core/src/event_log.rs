//! Fixed-capacity kernel event log.
//!
//! This module owns event sequencing and append behavior. It never allocates
//! and returns explicit capacity errors when the log is full.

use crate::{Event, KernelCore, KernelError};

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
    pub fn events(&self) -> &[Event] {
        &self.events[..self.event_len]
    }

    pub fn has_event_capacity(&self, needed: usize) -> bool {
        EVENTS.saturating_sub(self.event_len) >= needed
    }

    pub(crate) fn ensure_event_slots(&self, needed: usize) -> Result<(), KernelError> {
        if !self.has_event_capacity(needed) {
            Err(KernelError::EventLogFull)
        } else {
            Ok(())
        }
    }

    pub(crate) fn record(&mut self, event: Event) -> Result<Event, KernelError> {
        self.ensure_event_slots(1)?;

        let mut event = event;
        event.sequence = self.next_sequence;
        self.next_sequence += 1;
        self.events[self.event_len] = event;
        self.event_len += 1;
        Ok(event)
    }
}
