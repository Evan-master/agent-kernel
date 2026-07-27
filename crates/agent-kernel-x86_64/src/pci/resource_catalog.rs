//! Fixed-capacity PCI function resource catalog.
//!
//! This architecture-layer catalog probes Type-0 functions in stable BDF
//! order, retains restored BAR sets, and converts fully assigned functions into
//! architecture-neutral Driver Resource Tree specifications.

use agent_kernel_core::{
    DriverEndpointDescriptor, DriverResourceTreeSpec, ResourceKind, DRIVER_RESOURCE_REGION_CAPACITY,
};

use super::{
    probe_pci_function_bars, PciBarKind, PciBarProbeError, PciBarSet, PciConfigMutationAccess,
    PciFunction, PciFunctionAddress, PciInventory,
};

const TYPE_ZERO_HEADER: u8 = 0;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciFunctionResources {
    function: PciFunction,
    bars: PciBarSet,
}

impl PciFunctionResources {
    pub const fn function(self) -> PciFunction {
        self.function
    }

    pub const fn bars(self) -> PciBarSet {
        self.bars
    }

    pub fn driver_resource_spec(self) -> Option<DriverResourceTreeSpec> {
        if !self.bars.all_assigned() {
            return None;
        }
        let mut regions = [None; DRIVER_RESOURCE_REGION_CAPACITY];
        for bar in self.bars.bars() {
            let descriptor = match bar.kind() {
                PciBarKind::Io => {
                    let end = bar.end()?;
                    if end > u64::from(u16::MAX) {
                        return None;
                    }
                    DriverEndpointDescriptor::port(bar.base(), bar.size())
                }
                PciBarKind::MemoryBelowOneMegabyte { .. }
                | PciBarKind::Memory32 { .. }
                | PciBarKind::Memory64 { .. } => {
                    DriverEndpointDescriptor::mmio(bar.base(), bar.size())
                }
            };
            regions[usize::from(bar.index().number())] = Some(descriptor);
        }
        let root_kind = if self.function.class().is_network_controller() {
            ResourceKind::Network
        } else {
            ResourceKind::Device
        };
        Some(DriverResourceTreeSpec::new(root_kind, regions))
    }

    const EMPTY: Self = Self {
        function: PciFunction::EMPTY,
        bars: PciBarSet::EMPTY,
    };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciResourceCatalogError {
    Probe {
        address: PciFunctionAddress,
        error: PciBarProbeError,
    },
    CatalogFull {
        capacity: usize,
        address: PciFunctionAddress,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PciResourceCatalog<const CAPACITY: usize> {
    functions: [PciFunctionResources; CAPACITY],
    len: usize,
}

impl<const CAPACITY: usize> PciResourceCatalog<CAPACITY> {
    const fn new() -> Self {
        Self {
            functions: [PciFunctionResources::EMPTY; CAPACITY],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn functions(&self) -> &[PciFunctionResources] {
        &self.functions[..self.len]
    }

    pub fn find(&self, address: PciFunctionAddress) -> Option<PciFunctionResources> {
        self.functions()
            .iter()
            .copied()
            .find(|resources| resources.function().address() == address)
    }

    pub fn claim_candidate(&self) -> Option<PciFunctionResources> {
        self.functions()
            .iter()
            .copied()
            .find(|resources| resources.driver_resource_spec().is_some())
    }

    fn push(&mut self, resources: PciFunctionResources) -> Result<(), PciResourceCatalogError> {
        let Some(slot) = self.functions.get_mut(self.len) else {
            return Err(PciResourceCatalogError::CatalogFull {
                capacity: CAPACITY,
                address: resources.function().address(),
            });
        };
        *slot = resources;
        self.len += 1;
        Ok(())
    }
}

pub fn probe_pci_resource_catalog<
    A: PciConfigMutationAccess,
    const FUNCTIONS: usize,
    const CAPACITY: usize,
>(
    access: &mut A,
    inventory: &PciInventory<FUNCTIONS>,
) -> Result<PciResourceCatalog<CAPACITY>, PciResourceCatalogError> {
    let mut catalog = PciResourceCatalog::new();
    for function in inventory.functions() {
        if function.header_type() != TYPE_ZERO_HEADER {
            continue;
        }
        let bars = probe_pci_function_bars(access, function.address(), function.header_type())
            .map_err(|error| PciResourceCatalogError::Probe {
                address: function.address(),
                error,
            })?;
        catalog.push(PciFunctionResources {
            function: *function,
            bars,
        })?;
    }
    Ok(catalog)
}
