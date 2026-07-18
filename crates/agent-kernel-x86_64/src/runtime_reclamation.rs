//! Fixed-capacity runtime-memory reclamation plans for fault containment.
//!
//! This architecture-library module combines one compatibility-page binding
//! and bounded region bindings into deterministic cleanup order. It owns only
//! copyable identity; semantic retirement, page-table mutation, and physical
//! frame clearing remain in bare-metal adapters.

mod evidence;

use agent_kernel_core::{CapabilityId, MemoryCellId, ResourceId};

use crate::{
    runtime_page::RuntimePageBinding,
    runtime_region::{RuntimeRegionBinding, RUNTIME_REGION_CAPACITY},
};

pub use evidence::{RuntimeReclamationEvidence, RuntimeReclamationLog};

pub const RUNTIME_RECLAMATION_CAPACITY: usize = 1 + RUNTIME_REGION_CAPACITY;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeMemoryKind {
    Page,
    Region,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeReclamationCandidate {
    Page(RuntimePageBinding),
    Region(RuntimeRegionBinding),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeReclamationPlan {
    entries: [Option<RuntimeReclamationCandidate>; RUNTIME_RECLAMATION_CAPACITY],
    len: u8,
}

impl RuntimeReclamationPlan {
    pub fn new(
        page: Option<RuntimePageBinding>,
        regions: [Option<RuntimeRegionBinding>; RUNTIME_REGION_CAPACITY],
    ) -> Option<Self> {
        let mut plan = Self {
            entries: [None; RUNTIME_RECLAMATION_CAPACITY],
            len: 0,
        };
        if let Some(binding) = page {
            plan.push(RuntimeReclamationCandidate::Page(binding))?;
        }
        for binding in regions.into_iter().flatten() {
            plan.push(RuntimeReclamationCandidate::Region(binding))?;
        }
        Some(plan)
    }

    pub const fn len(self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    pub fn get(self, index: usize) -> Option<RuntimeReclamationCandidate> {
        if index < self.len() {
            self.entries[index]
        } else {
            None
        }
    }

    fn push(&mut self, candidate: RuntimeReclamationCandidate) -> Option<()> {
        if self.len() == RUNTIME_RECLAMATION_CAPACITY
            || candidate.resource().raw() == 0
            || candidate.capability().raw() == 0
            || candidate.cell().raw() == 0
            || candidate.generation() == 0
            || candidate.page_count() == 0
            || (0..self.len()).any(|index| {
                self.entries[index].is_some_and(|existing| {
                    existing.resource() == candidate.resource()
                        || existing.cell() == candidate.cell()
                })
            })
        {
            return None;
        }
        self.entries[self.len()] = Some(candidate);
        self.len = self.len.checked_add(1)?;
        Some(())
    }
}

impl RuntimeReclamationCandidate {
    pub const fn kind(self) -> RuntimeMemoryKind {
        match self {
            Self::Page(_) => RuntimeMemoryKind::Page,
            Self::Region(_) => RuntimeMemoryKind::Region,
        }
    }

    pub const fn resource(self) -> ResourceId {
        match self {
            Self::Page(binding) => binding.resource(),
            Self::Region(binding) => binding.resource(),
        }
    }

    pub const fn capability(self) -> CapabilityId {
        match self {
            Self::Page(binding) => binding.capability(),
            Self::Region(binding) => binding.capability(),
        }
    }

    pub const fn cell(self) -> MemoryCellId {
        match self {
            Self::Page(binding) => binding.cell(),
            Self::Region(binding) => binding.cell(),
        }
    }

    pub const fn generation(self) -> u64 {
        match self {
            Self::Page(binding) => binding.generation(),
            Self::Region(binding) => binding.generation(),
        }
    }

    pub const fn page_count(self) -> usize {
        match self {
            Self::Page(_) => 1,
            Self::Region(binding) => binding.page_count(),
        }
    }
}
