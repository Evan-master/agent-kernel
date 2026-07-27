//! BSP-owned native PCI configuration discovery.
//!
//! This boot adapter creates the only Configuration Mechanism 1 owner while
//! interrupts are disabled, retains restored BAR observations, and validates
//! an exact Function claim before handing it to the Driver flow.

use agent_kernel_x86_64::{
    pci::{
        discover_pci_functions, probe_pci_resource_catalog, PciConfigMechanismOne,
        PciConfigMechanismOneError, PciDiscoveryError, PciFunctionClaim, PciFunctionResources,
        PciInventory, PciResourceCatalog, PciResourceCatalogError,
    },
    NativePortIo,
};

use crate::pci_serial_profile;

use super::{SmpBootError, SmpBootstrap};

pub(super) const PCI_FUNCTION_CAPACITY: usize = 256;
pub(super) type BootPciInventory = PciInventory<PCI_FUNCTION_CAPACITY>;
pub(super) type BootPciResourceCatalog = PciResourceCatalog<PCI_FUNCTION_CAPACITY>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PciBootError {
    Configuration(PciConfigMechanismOneError),
    Discovery(PciDiscoveryError),
    InventoryUnavailable,
    ResourceCatalog(PciResourceCatalogError),
    ClaimTargetUnavailable,
    ClaimCandidateMismatch,
    InterruptProfileMismatch,
}

pub(super) fn discover() -> Result<BootPciInventory, PciBootError> {
    // SAFETY: this runs on the BSP with IF clear before AP startup. The value is
    // bound immediately to the fixed PCI configuration adapter.
    let io = unsafe { NativePortIo::new() };
    let mut config = PciConfigMechanismOne::new(io);
    config.probe().map_err(PciBootError::Configuration)?;
    discover_pci_functions(&mut config).map_err(PciBootError::Discovery)
}

pub(super) fn probe_resources(
    inventory: &BootPciInventory,
) -> Result<BootPciResourceCatalog, PciBootError> {
    // SAFETY: this one-shot owner runs on the BSP with IF clear before AP
    // startup and keeps mutation inside the reversible BAR probe.
    let io = unsafe { NativePortIo::new() };
    let mut config = PciConfigMechanismOne::new(io);
    config.probe().map_err(PciBootError::Configuration)?;
    probe_pci_resource_catalog(&mut config, inventory).map_err(PciBootError::ResourceCatalog)
}

impl SmpBootstrap {
    pub(crate) fn prepare_pci_inventory(&mut self) -> Result<usize, SmpBootError> {
        if self.pci_inventory.is_some() {
            return Err(SmpBootError::PciAlreadyDiscovered);
        }
        let inventory = discover().map_err(SmpBootError::Pci)?;
        let function_count = inventory.len();
        self.pci_inventory = Some(inventory);
        Ok(function_count)
    }

    pub(crate) const fn pci_inventory(&self) -> Option<&BootPciInventory> {
        self.pci_inventory.as_ref()
    }

    pub(crate) fn prepare_pci_intx_route(&mut self) -> Result<(), SmpBootError> {
        let selector = pci_serial_profile::selector()
            .ok_or(SmpBootError::Pci(PciBootError::InterruptProfileMismatch))?;
        let function = self
            .pci_inventory
            .as_ref()
            .and_then(|inventory| inventory.find(selector.address()))
            .filter(|function| selector.matches(*function))
            .ok_or(SmpBootError::Pci(PciBootError::InterruptProfileMismatch))?;
        if function.interrupt_line() != pci_serial_profile::INTERRUPT_LINE
            || function.interrupt_pin() != Some(pci_serial_profile::INTERRUPT_PIN)
        {
            return Err(SmpBootError::Pci(PciBootError::InterruptProfileMismatch));
        }
        self.io_apic_routing
            .as_mut()
            .ok_or(SmpBootError::InvalidLocalApicMapping)?
            .prepare_pci_intx(&self.topology, function.interrupt_line())
            .map_err(SmpBootError::IoApicRouting)
    }

    pub(crate) fn prepare_pci_resources(&mut self) -> Result<usize, SmpBootError> {
        if self.pci_resources.is_some() {
            return Err(SmpBootError::PciResourcesAlreadyProbed);
        }
        let inventory = self
            .pci_inventory
            .as_ref()
            .ok_or(SmpBootError::Pci(PciBootError::InventoryUnavailable))?;
        let resources = probe_resources(inventory).map_err(SmpBootError::Pci)?;
        let function_count = resources.len();
        self.pci_resources = Some(resources);
        Ok(function_count)
    }

    pub(crate) const fn pci_resources(&self) -> Option<&BootPciResourceCatalog> {
        self.pci_resources.as_ref()
    }

    pub(crate) fn pci_driver_candidate(&self) -> Option<PciFunctionResources> {
        self.pci_resources
            .as_ref()?
            .claim_candidate_for(pci_serial_profile::selector()?)
    }

    pub(crate) fn install_pci_claim(
        &mut self,
        claim: PciFunctionClaim,
    ) -> Result<(), SmpBootError> {
        if self.pci_claim.is_some() {
            return Err(SmpBootError::PciClaimAlreadyInstalled);
        }
        let expected = self
            .pci_driver_candidate()
            .ok_or(SmpBootError::Pci(PciBootError::ClaimTargetUnavailable))?;
        if claim.function() != expected.function() || claim.bars() != expected.bars() {
            return Err(SmpBootError::Pci(PciBootError::ClaimCandidateMismatch));
        }
        self.pci_claim = Some(claim);
        Ok(())
    }

    pub(crate) const fn pci_claim(&self) -> Option<PciFunctionClaim> {
        self.pci_claim
    }
}
