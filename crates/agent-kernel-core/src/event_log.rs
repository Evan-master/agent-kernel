//! Fixed-capacity kernel event log.
//!
//! This module owns event sequencing and append behavior. It never allocates
//! and returns explicit capacity errors when the log is full.

use crate::{Event, KernelCore, KernelError};

impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize>
    KernelCore<RESOURCES, CAPS, EVENTS>
{
    pub(crate) fn record(&mut self, event: Event) -> Result<Event, KernelError> {
        if self.event_len >= EVENTS {
            return Err(KernelError::EventLogFull);
        }

        let mut event = event;
        event.sequence = self.next_sequence;
        self.next_sequence += 1;
        self.events[self.event_len] = event;
        self.event_len += 1;
        Ok(event)
    }
}
