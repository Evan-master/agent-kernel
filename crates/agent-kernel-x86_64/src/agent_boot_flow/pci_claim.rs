//! Native PCI Function claim installation and retained boot evidence.
//!
//! This BSP-only adapter projects one restored Function catalog entry into the
//! kernel's atomic Driver Resource Tree, verifies every resulting authority,
//! and retains the BDF-to-Resource mapping outside Ring-3.

use agent_kernel_core::{Operation, OperationSet, ResourceKind};
use agent_kernel_x86_64::pci::PciFunctionClaim;

use crate::{serial_write_line, smp_boot::SmpBootstrap, X86BootedKernel};

pub(super) fn install(
    booted: &mut X86BootedKernel,
    smp: &mut SmpBootstrap,
) -> Option<PciFunctionClaim> {
    let candidate = smp.pci_driver_candidate()?;
    let spec = candidate.driver_resource_spec()?;
    let report = *booted.report();
    let tree = booted
        .kernel_mut()
        .sys_create_driver_resource_tree(
            report.bootstrap_agent,
            Some((report.bootstrap_resource, report.bootstrap_capability)),
            claim_operations(),
            spec,
        )
        .ok()?;
    let claim = PciFunctionClaim::new(candidate, tree).ok()?;
    if !claim_matches_kernel(booted, claim, spec.root_kind()) {
        return None;
    }
    smp.install_pci_claim(claim).ok()?;
    if smp.pci_claim() != Some(claim) {
        return None;
    }
    serial_write_line("AGENT_KERNEL_PCI_FUNCTION_CLAIM_OK");
    serial_write_line("AGENT_KERNEL_PCI_CAPABILITY_BOUNDARY_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_TARGET_OK");
    Some(claim)
}

fn claim_matches_kernel(
    booted: &X86BootedKernel,
    claim: PciFunctionClaim,
    root_kind: ResourceKind,
) -> bool {
    let kernel = booted.kernel();
    let report = *booted.report();
    let root = claim.root();
    let Some(root_record) = kernel
        .resources()
        .iter()
        .find(|resource| resource.id == root.resource)
    else {
        return false;
    };
    let Ok(root_capability) = kernel.capability(root.capability) else {
        return false;
    };
    if root_record.kind != root_kind
        || root_record.parent != Some(report.bootstrap_resource)
        || root_record.owner != Some(report.bootstrap_agent)
        || root_capability.agent != report.bootstrap_agent
        || root_capability.resource != root.resource
        || root_capability.operations != claim_operations()
    {
        return false;
    }

    for bar in claim.bars().bars() {
        let Some(region) = claim.bar_region(bar.index()) else {
            return false;
        };
        let Some(resource) = kernel
            .resources()
            .iter()
            .find(|resource| resource.id == region.resource())
        else {
            return false;
        };
        let Ok(capability) = kernel.capability(region.capability()) else {
            return false;
        };
        let Ok(endpoint) = kernel.driver_endpoint(region.resource()) else {
            return false;
        };
        if resource.kind != ResourceKind::Device
            || resource.parent != Some(root.resource)
            || resource.owner != Some(report.bootstrap_agent)
            || capability.agent != report.bootstrap_agent
            || capability.resource != region.resource()
            || capability.operations != claim_operations()
            || endpoint.descriptor != region.descriptor()
        {
            return false;
        }
    }
    true
}

const fn claim_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Checkpoint)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}
