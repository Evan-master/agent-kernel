//! Bounded PCI configuration discovery for the native x86_64 boot path.
//!
//! This architecture layer owns read-only segment-zero configuration access,
//! deterministic BDF scanning, and an immutable fixed-capacity function
//! inventory. Device mutation and Agent-visible authority are separate stages.

mod config;
mod inventory;
mod types;

pub use config::{PciConfigAccess, PciConfigIo, PciConfigMechanismOne, PciConfigMechanismOneError};
pub use inventory::{discover_pci_functions, PciDiscoveryError, PciInventory};
pub use types::{PciClassCode, PciConfigRegister, PciFunction, PciFunctionAddress};
