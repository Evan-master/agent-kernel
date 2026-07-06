//! Kernel-owned memory cell model for native agent state.
//!
//! This module belongs to `agent-kernel-core`. It defines fixed-width memory
//! values and copyable memory cell records for the no_std memory store. It does
//! not model virtual addresses, byte buffers, host files, or heap allocation.

use crate::{AgentId, MemoryCellId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MemoryValue {
    pub words: [u64; 4],
}

impl MemoryValue {
    pub const fn new(words: [u64; 4]) -> Self {
        Self { words }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MemoryCellRecord {
    pub id: MemoryCellId,
    pub resource: ResourceId,
    pub creator: AgentId,
    pub last_writer: AgentId,
    pub value: MemoryValue,
    pub revision: u64,
}

impl MemoryCellRecord {
    pub(crate) const fn empty() -> Self {
        Self {
            id: MemoryCellId::new(0),
            resource: ResourceId::new(0),
            creator: AgentId::new(0),
            last_writer: AgentId::new(0),
            value: MemoryValue::new([0, 0, 0, 0]),
            revision: 0,
        }
    }
}
