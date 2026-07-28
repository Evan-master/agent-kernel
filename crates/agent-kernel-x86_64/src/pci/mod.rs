//! Bounded PCI configuration discovery for the native x86_64 boot path.
//!
//! This architecture layer owns read-only segment-zero configuration access,
//! deterministic BDF scanning, and an immutable fixed-capacity function
//! inventory. Device mutation and Agent-visible authority are separate stages.

mod bar;
mod bar_probe;
mod capability;
mod claim;
mod command;
mod config;
mod config_field;
mod inventory;
mod msi;
mod msi_message;
mod msix;
mod msix_error;
mod msix_table;
mod resource_catalog;
mod types;
mod virtio_capability;

pub use bar::{PciBar, PciBarIndex, PciBarKind, PciBarSet, PCI_BAR_CAPACITY};
pub use bar_probe::{probe_pci_function_bars, PciBarProbeError};
pub use capability::{
    discover_pci_capabilities, discover_pci_capabilities_bounded, PciCapability,
    PciCapabilityError, PciCapabilityList, PCI_CAPABILITY_CAPACITY, PCI_CAPABILITY_ID_MSI,
    PCI_CAPABILITY_ID_MSIX, PCI_CAPABILITY_ID_VENDOR_SPECIFIC,
};
pub use claim::{PciFunctionClaim, PciFunctionClaimError};
pub use command::{PciCommandGate, PciCommandGateError, PciCommandState};
pub use config::{
    PciConfigAccess, PciConfigIo, PciConfigMechanismOne, PciConfigMechanismOneError,
    PciConfigMutationAccess, PciConfigWriteIo,
};
pub use inventory::{discover_pci_functions, PciDiscoveryError, PciInventory};
pub use msi::{MsiCapability, MsiError, MsiRegister};
pub use msi_message::{XapicMsiMessage, XapicMsiMessageError};
pub use msix::MsixCapability;
pub use msix_error::{MsixDescriptor, MsixError, MsixTableField};
pub use msix_table::{
    program_msix_table_entry, MsixTableAccess, MsixTableRegion, VolatileMsixTable,
};
pub use resource_catalog::{
    probe_pci_resource_catalog, PciFunctionResources, PciResourceCatalog, PciResourceCatalogError,
};
pub use types::{
    PciClassCode, PciConfigRegister, PciFunction, PciFunctionAddress, PciFunctionSelector,
    PciInterruptPin,
};
pub use virtio_capability::{
    VirtioPciBarRegion, VirtioPciCapability, VirtioPciCapabilityError, VirtioPciCapabilityKind,
};
