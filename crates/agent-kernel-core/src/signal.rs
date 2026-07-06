//! Agent-native wait signal model.
//!
//! This module belongs to `agent-kernel-core`. It defines compact signal keys,
//! waiter records, and signal outcomes for fixed-capacity task blocking and
//! wakeup. It performs no allocation, host waiting, async runtime work, or I/O.

use crate::{AgentId, Event, ResourceId, TaskId, WaiterId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SignalKey(u64);

impl SignalKey {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WaiterRecord {
    pub id: WaiterId,
    pub task: TaskId,
    pub agent: AgentId,
    pub resource: ResourceId,
    pub signal: SignalKey,
    pub active: bool,
}

impl WaiterRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: WaiterId::new(0),
            task: TaskId::new(0),
            agent: AgentId::new(0),
            resource: ResourceId::new(0),
            signal: SignalKey::new(0),
            active: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SignalOutcome {
    pub signal_event: Event,
    pub woken_task: Option<TaskId>,
    pub wake_event: Option<Event>,
}
