//! Copyable transaction tokens for the pure runtime frame-pool ledger.
//!
//! Constructors and raw transaction identity stay private to the parent
//! module. Public accessors expose only the ownership evidence needed by the
//! bare-metal pool and host contract tests.

use agent_kernel_core::{AgentId, MemoryCellId, ResourceId};

use super::MAX_RUNTIME_REGION_PAGES;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFrameReservation {
    agent: AgentId,
    resource: ResourceId,
    page_count: u8,
    indices: [u8; MAX_RUNTIME_REGION_PAGES],
    transaction: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFrameBinding {
    agent: AgentId,
    resource: ResourceId,
    cell: MemoryCellId,
    generation: u64,
    page_count: u8,
    indices: [u8; MAX_RUNTIME_REGION_PAGES],
    transaction: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFrameRelease(RuntimeFrameBinding);

impl RuntimeFrameReservation {
    pub(super) fn new(
        agent: AgentId,
        resource: ResourceId,
        page_count: usize,
        indices: [u8; MAX_RUNTIME_REGION_PAGES],
        transaction: u64,
    ) -> Self {
        Self {
            agent,
            resource,
            page_count: page_count as u8,
            indices,
            transaction,
        }
    }

    pub const fn agent(self) -> AgentId {
        self.agent
    }

    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn page_count(self) -> usize {
        self.page_count as usize
    }

    pub fn frame_index(self, page: usize) -> Option<usize> {
        (page < self.page_count()).then(|| usize::from(self.indices[page]))
    }

    pub(super) const fn indices(self) -> [u8; MAX_RUNTIME_REGION_PAGES] {
        self.indices
    }

    pub(super) const fn transaction(self) -> u64 {
        self.transaction
    }
}

impl RuntimeFrameBinding {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        agent: AgentId,
        resource: ResourceId,
        cell: MemoryCellId,
        generation: u64,
        page_count: usize,
        indices: [u8; MAX_RUNTIME_REGION_PAGES],
        transaction: u64,
    ) -> Self {
        Self {
            agent,
            resource,
            cell,
            generation,
            page_count: page_count as u8,
            indices,
            transaction,
        }
    }

    pub const fn agent(self) -> AgentId {
        self.agent
    }

    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn cell(self) -> MemoryCellId {
        self.cell
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn page_count(self) -> usize {
        self.page_count as usize
    }

    pub fn frame_index(self, page: usize) -> Option<usize> {
        (page < self.page_count()).then(|| usize::from(self.indices[page]))
    }

    pub(super) const fn release(self) -> RuntimeFrameRelease {
        RuntimeFrameRelease(self)
    }
}

impl RuntimeFrameRelease {
    pub const fn agent(self) -> AgentId {
        self.0.agent()
    }

    pub const fn resource(self) -> ResourceId {
        self.0.resource()
    }

    pub const fn cell(self) -> MemoryCellId {
        self.0.cell()
    }

    pub const fn generation(self) -> u64 {
        self.0.generation()
    }

    pub const fn page_count(self) -> usize {
        self.0.page_count()
    }

    pub fn frame_index(self, page: usize) -> Option<usize> {
        self.0.frame_index(page)
    }

    pub(super) const fn indices(self) -> [u8; MAX_RUNTIME_REGION_PAGES] {
        self.0.indices
    }

    pub(super) const fn binding(self) -> RuntimeFrameBinding {
        self.0
    }
}
