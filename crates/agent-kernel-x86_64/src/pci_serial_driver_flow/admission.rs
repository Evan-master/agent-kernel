//! Reclaimed Agent identity, image slot, Capability, and Driver Binding setup.
//!
//! This boot-only phase reuses terminal fixed-capacity records through public
//! syscalls and launches one BAR-scoped Driver entry before hardware access.

use agent_kernel_core::{
    AgentEntryKind, AgentImageId, AgentImageKind, AgentImageStatus, CapabilityId, DriverBindingId,
    DriverEndpointRecord, DriverResourceRegion, Operation, OperationSet,
};

use super::{DRIVER, RECLAIMABLE_IMAGE};
use crate::{boot_agent_images::BootPciSerialDriverImage, serial_write_line, X86BootedKernel};

pub(super) struct PciSerialAdmission {
    pub(super) capability: CapabilityId,
    pub(super) binding: DriverBindingId,
    pub(super) endpoint: DriverEndpointRecord,
    pub(super) image: AgentImageId,
}

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    region: DriverResourceRegion,
    contract: BootPciSerialDriverImage,
) -> Option<PciSerialAdmission> {
    let report = *booted.report();
    let reclaimable = booted.kernel().agent_image(RECLAIMABLE_IMAGE).ok()?;
    if reclaimable.owner != report.bootstrap_agent
        || reclaimable.resource != report.bootstrap_resource
        || reclaimable.kind != AgentImageKind::Worker
        || reclaimable.status != AgentImageStatus::Pending
        || booted.kernel().agent_entry(DRIVER).is_ok()
    {
        return None;
    }
    booted
        .kernel_mut()
        .sys_retire_agent_image(
            report.bootstrap_agent,
            report.bootstrap_capability,
            RECLAIMABLE_IMAGE,
        )
        .ok()?;
    let retirement = booted
        .kernel_mut()
        .sys_retire_agent_image_record(
            report.bootstrap_agent,
            report.bootstrap_capability,
            RECLAIMABLE_IMAGE,
        )
        .ok()?;
    let retired = retirement.record();
    if retired.id != RECLAIMABLE_IMAGE
        || retired.owner != report.bootstrap_agent
        || retired.resource != report.bootstrap_resource
        || retired.kind != AgentImageKind::Worker
        || retired.status != AgentImageStatus::Retired
        || retirement.actor() != report.bootstrap_agent
        || retirement.authority() != report.bootstrap_capability
    {
        return None;
    }

    let operations = driver_operations();
    let capability = booted
        .kernel_mut()
        .sys_derive_capability(
            report.bootstrap_agent,
            region.capability(),
            DRIVER,
            operations,
        )
        .ok()?;
    let capability_record = booted.kernel().capability(capability).ok()?;
    if capability_record.agent != DRIVER
        || capability_record.resource != region.resource()
        || capability_record.operations != operations
        || capability_record.parent != Some(region.capability())
        || capability_record.revoked
    {
        return None;
    }

    let image = booted
        .kernel_mut()
        .sys_register_agent_image(
            report.bootstrap_agent,
            region.capability(),
            region.resource(),
            AgentImageKind::Driver,
            contract.digest(),
            1,
            1,
        )
        .ok()?;
    booted
        .kernel_mut()
        .sys_verify_agent_image(report.bootstrap_agent, region.capability(), image)
        .ok()?;
    booted
        .kernel_mut()
        .sys_launch_agent(
            DRIVER,
            capability,
            region.resource(),
            image,
            AgentEntryKind::Driver,
            None,
        )
        .ok()?;
    let binding = booted
        .kernel_mut()
        .sys_bind_driver(
            report.bootstrap_agent,
            region.capability(),
            region.resource(),
            DRIVER,
        )
        .ok()?;
    let endpoint = booted.kernel().driver_endpoint(region.resource()).ok()?;
    let image_record = booted.kernel().agent_image(image).ok()?;
    if image_record.kind != AgentImageKind::Driver
        || image_record.digest != contract.digest()
        || image_record.status != AgentImageStatus::Verified
        || endpoint.resource != region.resource()
        || endpoint.descriptor != region.descriptor()
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_PCI_SERIAL_AGENT_REUSED_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_CAPABILITY_OK");
    Some(PciSerialAdmission {
        capability,
        binding,
        endpoint,
        image,
    })
}

const fn driver_operations() -> OperationSet {
    OperationSet::only(Operation::Observe).with(Operation::Act)
}
