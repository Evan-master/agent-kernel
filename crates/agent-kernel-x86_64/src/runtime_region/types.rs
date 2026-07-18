//! Copyable virtual-region reservation, binding, and cleanup tokens.
//!
//! Constructors and transaction identity stay visible only to the parent
//! ledger. Bare-metal adapters consume the public ownership and range
//! accessors after validating semantic kernel records.

use agent_kernel_core::{MemoryCellId, ResourceId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRegionReservation {
    resource: ResourceId,
    start_slot: u8,
    page_count: u8,
    generation: u64,
    transaction: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRegionBinding {
    resource: ResourceId,
    cell: MemoryCellId,
    start_slot: u8,
    page_count: u8,
    generation: u64,
    transaction: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRegionRelease(RuntimeRegionBinding);

impl RuntimeRegionReservation {
    pub(super) fn new(
        resource: ResourceId,
        start_slot: usize,
        page_count: usize,
        generation: u64,
        transaction: u64,
    ) -> Self {
        Self {
            resource,
            start_slot: start_slot as u8,
            page_count: page_count as u8,
            generation,
            transaction,
        }
    }

    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn start_slot(self) -> usize {
        self.start_slot as usize
    }

    pub const fn page_count(self) -> usize {
        self.page_count as usize
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub(super) const fn transaction(self) -> u64 {
        self.transaction
    }
}

impl RuntimeRegionBinding {
    pub(super) fn new(
        resource: ResourceId,
        cell: MemoryCellId,
        start_slot: usize,
        page_count: usize,
        generation: u64,
        transaction: u64,
    ) -> Self {
        Self {
            resource,
            cell,
            start_slot: start_slot as u8,
            page_count: page_count as u8,
            generation,
            transaction,
        }
    }

    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn cell(self) -> MemoryCellId {
        self.cell
    }

    pub const fn start_slot(self) -> usize {
        self.start_slot as usize
    }

    pub const fn page_count(self) -> usize {
        self.page_count as usize
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub(super) const fn release(self) -> RuntimeRegionRelease {
        RuntimeRegionRelease(self)
    }
}

impl RuntimeRegionRelease {
    pub const fn resource(self) -> ResourceId {
        self.0.resource()
    }

    pub const fn cell(self) -> MemoryCellId {
        self.0.cell()
    }

    pub const fn start_slot(self) -> usize {
        self.0.start_slot()
    }

    pub const fn page_count(self) -> usize {
        self.0.page_count()
    }

    pub const fn generation(self) -> u64 {
        self.0.generation()
    }

    pub(super) const fn transaction(self) -> u64 {
        self.0.transaction
    }
}
