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
    >
{
    pub(crate) fn ensure_event_slots(&self, needed: usize) -> Result<(), KernelError> {
        if EVENTS.saturating_sub(self.event_len) < needed {
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
