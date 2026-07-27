//! Bounded PCI configuration discovery for the native x86_64 boot path.
//!
//! This architecture layer owns read-only segment-zero configuration access,
//! deterministic BDF scanning, and an immutable fixed-capacity function
//! inventory. Device mutation and Agent-visible authority are separate stages.

mod bar;
mod bar_probe;
mod claim;
mod config;
mod inventory;
mod resource_catalog;
mod types;

pub use bar::{PciBar, PciBarIndex, PciBarKind, PciBarSet, PCI_BAR_CAPACITY};
pub use bar_probe::{probe_pci_function_bars, PciBarProbeError};
pub use claim::{PciFunctionClaim, PciFunctionClaimError};
pub use config::{
    PciConfigAccess, PciConfigIo, PciConfigMechanismOne, PciConfigMechanismOneError,
    PciConfigMutationAccess, PciConfigWriteIo,
};
pub use inventory::{discover_pci_functions, PciDiscoveryError, PciInventory};
pub use resource_catalog::{
    probe_pci_resource_catalog, PciFunctionResources, PciResourceCatalog, PciResourceCatalogError,
};
pub use types::{
    PciClassCode, PciConfigRegister, PciFunction, PciFunctionAddress, PciFunctionSelector,
};
