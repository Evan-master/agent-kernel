//! Ordered proof records for completed runtime-memory fault cleanup.
//!
//! This architecture-library child stores bounded semantic identity and the
//! first and last observed frame words. Records follow plan order and reject
//! duplicates; physical zero-state validation remains with the memory pool.

use agent_kernel_core::{CapabilityId, MemoryCellId, ResourceId};

use super::{
    RuntimeMemoryKind, RuntimeReclamationCandidate, RuntimeReclamationPlan,
    RUNTIME_RECLAMATION_CAPACITY,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeReclamationEvidence {
    kind: RuntimeMemoryKind,
    resource: ResourceId,
    capability: CapabilityId,
    cell: MemoryCellId,
    page_count: u8,
    generation: u64,
    first: u64,
    last: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeReclamationLog {
    entries: [Option<RuntimeReclamationEvidence>; RUNTIME_RECLAMATION_CAPACITY],
    len: u8,
}

impl RuntimeReclamationLog {
    pub const fn new() -> Self {
        Self {
            entries: [None; RUNTIME_RECLAMATION_CAPACITY],
            len: 0,
        }
    }

    pub const fn len(self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    pub fn get(self, index: usize) -> Option<RuntimeReclamationEvidence> {
        if index < self.len() {
            self.entries[index]
        } else {
            None
        }
    }

    pub fn record(
        &mut self,
        candidate: RuntimeReclamationCandidate,
        first: u64,
        last: u64,
    ) -> bool {
        if self.len() == RUNTIME_RECLAMATION_CAPACITY
            || (0..self.len())
                .any(|index| self.entries[index].is_some_and(|entry| entry.matches(candidate)))
        {
            return false;
        }
        self.entries[self.len()] = Some(RuntimeReclamationEvidence {
            kind: candidate.kind(),
            resource: candidate.resource(),
            capability: candidate.capability(),
            cell: candidate.cell(),
            page_count: candidate.page_count() as u8,
            generation: candidate.generation(),
            first,
            last,
        });
        let Some(next) = self.len.checked_add(1) else {
            return false;
        };
        self.len = next;
        true
    }

    pub fn matches_plan(self, plan: RuntimeReclamationPlan) -> bool {
        self.len() == plan.len()
            && (0..self.len()).all(|index| {
                self.get(index)
                    .zip(plan.get(index))
                    .is_some_and(|(evidence, candidate)| evidence.matches(candidate))
            })
    }
}

impl Default for RuntimeReclamationLog {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeReclamationEvidence {
    pub const fn kind(self) -> RuntimeMemoryKind {
        self.kind
    }

    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn capability(self) -> CapabilityId {
        self.capability
    }

    pub const fn cell(self) -> MemoryCellId {
        self.cell
    }

    pub const fn page_count(self) -> usize {
        self.page_count as usize
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn first(self) -> u64 {
        self.first
    }

    pub const fn last(self) -> u64 {
        self.last
    }

    const fn matches(self, candidate: RuntimeReclamationCandidate) -> bool {
        self.kind as u8 == candidate.kind() as u8
            && self.resource.raw() == candidate.resource().raw()
            && self.capability.raw() == candidate.capability().raw()
            && self.cell.raw() == candidate.cell().raw()
            && self.page_count as usize == candidate.page_count()
            && self.generation == candidate.generation()
    }
}
