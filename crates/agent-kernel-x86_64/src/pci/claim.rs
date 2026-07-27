//! Capability-bound PCI function claim mapping.
//!
//! This architecture-layer value binds each restored BAR slot to the exact
//! Resource, Capability, and immutable Driver Endpoint produced by one atomic
//! core transaction. It performs no hardware access.

use agent_kernel_core::{
    DriverResourceRegion, DriverResourceTree, ResourceCreateOutcome,
    DRIVER_RESOURCE_REGION_CAPACITY,
};

use super::{PciBarIndex, PciBarSet, PciFunction, PciFunctionResources};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciFunctionClaimError {
    FunctionNotClaimable,
    ResourceTreeMismatch { slot: u8 },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciFunctionClaim {
    resources: PciFunctionResources,
    tree: DriverResourceTree,
}

impl PciFunctionClaim {
    pub fn new(
        resources: PciFunctionResources,
        tree: DriverResourceTree,
    ) -> Result<Self, PciFunctionClaimError> {
        let spec = resources
            .driver_resource_spec()
            .ok_or(PciFunctionClaimError::FunctionNotClaimable)?;
        for slot in 0..DRIVER_RESOURCE_REGION_CAPACITY {
            let expected = spec.regions()[slot];
            let actual = tree.region(slot).map(|region| region.descriptor());
            if expected != actual {
                return Err(PciFunctionClaimError::ResourceTreeMismatch { slot: slot as u8 });
            }
        }
        Ok(Self { resources, tree })
    }

    pub const fn function(self) -> PciFunction {
        self.resources.function()
    }

    pub const fn bars(self) -> PciBarSet {
        self.resources.bars()
    }

    pub const fn root(self) -> ResourceCreateOutcome {
        self.tree.root()
    }

    pub fn bar_region(self, index: PciBarIndex) -> Option<DriverResourceRegion> {
        self.tree.region(usize::from(index.number()))
    }

    pub const fn tree(self) -> DriverResourceTree {
        self.tree
    }
}
