//! Fixed-capacity Driver Resource Tree request and outcome values.
//!
//! This core-layer module describes one driver-owned root Resource and up to
//! six physical endpoint-region children. It stores no state and performs no
//! authorization; `KernelCore` owns the atomic transaction.

use crate::{
    CapabilityId, DriverEndpointDescriptor, ResourceCreateOutcome, ResourceId, ResourceKind,
};

pub const DRIVER_RESOURCE_REGION_CAPACITY: usize = 6;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverResourceTreeSpec {
    root_kind: ResourceKind,
    regions: [Option<DriverEndpointDescriptor>; DRIVER_RESOURCE_REGION_CAPACITY],
}

impl DriverResourceTreeSpec {
    pub const fn new(
        root_kind: ResourceKind,
        regions: [Option<DriverEndpointDescriptor>; DRIVER_RESOURCE_REGION_CAPACITY],
    ) -> Self {
        Self { root_kind, regions }
    }

    pub const fn root_kind(self) -> ResourceKind {
        self.root_kind
    }

    pub const fn regions(
        &self,
    ) -> &[Option<DriverEndpointDescriptor>; DRIVER_RESOURCE_REGION_CAPACITY] {
        &self.regions
    }

    pub fn region_count(&self) -> usize {
        self.regions.iter().flatten().count()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverResourceRegion {
    slot: u8,
    resource: ResourceId,
    capability: CapabilityId,
    descriptor: DriverEndpointDescriptor,
}

impl DriverResourceRegion {
    pub(crate) const fn new(
        slot: u8,
        resource: ResourceId,
        capability: CapabilityId,
        descriptor: DriverEndpointDescriptor,
    ) -> Self {
        Self {
            slot,
            resource,
            capability,
            descriptor,
        }
    }

    pub const fn slot(self) -> u8 {
        self.slot
    }

    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn capability(self) -> CapabilityId {
        self.capability
    }

    pub const fn descriptor(self) -> DriverEndpointDescriptor {
        self.descriptor
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DriverResourceTree {
    root: ResourceCreateOutcome,
    regions: [Option<DriverResourceRegion>; DRIVER_RESOURCE_REGION_CAPACITY],
}

impl DriverResourceTree {
    pub(crate) const fn new(
        root: ResourceCreateOutcome,
        regions: [Option<DriverResourceRegion>; DRIVER_RESOURCE_REGION_CAPACITY],
    ) -> Self {
        Self { root, regions }
    }

    pub const fn root(self) -> ResourceCreateOutcome {
        self.root
    }

    pub fn region(&self, slot: usize) -> Option<DriverResourceRegion> {
        self.regions.get(slot).copied().flatten()
    }

    pub fn region_count(&self) -> usize {
        self.regions.iter().flatten().count()
    }

    pub const fn regions(
        &self,
    ) -> &[Option<DriverResourceRegion>; DRIVER_RESOURCE_REGION_CAPACITY] {
        &self.regions
    }
}
