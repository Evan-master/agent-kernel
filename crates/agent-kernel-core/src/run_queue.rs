//! Kernel-owned run queue entry model.
//!
//! This module belongs to `agent-kernel-core`. It defines the compact copyable
//! entry used by the fixed-capacity FIFO scheduler. It has no allocation or
//! host dependencies.

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
