//! Kernel-owned run queue entry model.
//!
//! This module belongs to `agent-kernel-core`. It defines the compact copyable
//! entry and two-phase dispatch permit used by the fixed-capacity FIFO
//! scheduler. It has no allocation or host dependencies.

use crate::{AgentId, TaskId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RunQueueEntry {
    pub task: TaskId,
    pub agent: AgentId,
}

impl RunQueueEntry {
    pub(crate) const fn empty() -> Self {
        Self {
            task: TaskId::new(0),
            agent: AgentId::new(0),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TaskDispatchPermit {
    entry: RunQueueEntry,
    quantum: u64,
    generation: u64,
}

impl TaskDispatchPermit {
    pub(crate) const fn new(entry: RunQueueEntry, quantum: u64, generation: u64) -> Self {
        Self {
            entry,
            quantum,
            generation,
        }
    }

    pub const fn entry(self) -> RunQueueEntry {
        self.entry
    }

    pub const fn quantum(self) -> u64 {
        self.quantum
    }

    pub(crate) const fn generation(self) -> u64 {
        self.generation
    }
}
