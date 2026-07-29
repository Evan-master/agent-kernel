//! Core admission for the V30 Network Driver Agent.

use agent_kernel_core::{
    AgentEntryKind, AgentImageId, AgentImageKind, CapabilityId, DriverBindingId,
    DriverEndpointDescriptor, Operation, OperationSet, ResourceCreateOutcome,
};

use crate::{boot_agent_images::BootNetworkDriverImage, serial_write_line, X86BootedKernel};

use super::DRIVER;

#[derive(Copy, Clone)]
pub(super) struct NetworkDriverAdmission {
    pub(super) capability: CapabilityId,
    pub(super) binding: DriverBindingId,
    pub(super) image: AgentImageId,
}

#[derive(Copy, Clone)]
pub(super) enum NetworkDriverAdmissionError {
    Agent,
    Capability,
    Endpoint,
    ImageRegistration,
    ImageVerification,
    Launch,
    Binding,
    Contract,
}

impl NetworkDriverAdmissionError {
    pub(super) const fn diagnostic_marker(self) -> &'static str {
        match self {
            Self::Agent => "AGENT_KERNEL_NATIVE_UDP_DRIVER_AGENT_ERROR",
            Self::Capability => "AGENT_KERNEL_NATIVE_UDP_DRIVER_CAPABILITY_ERROR",
            Self::Endpoint => "AGENT_KERNEL_NATIVE_UDP_DRIVER_ENDPOINT_ERROR",
            Self::ImageRegistration => "AGENT_KERNEL_NATIVE_UDP_DRIVER_IMAGE_REGISTRATION_ERROR",
            Self::ImageVerification => "AGENT_KERNEL_NATIVE_UDP_DRIVER_IMAGE_VERIFICATION_ERROR",
            Self::Launch => "AGENT_KERNEL_NATIVE_UDP_DRIVER_LAUNCH_ERROR",
            Self::Binding => "AGENT_KERNEL_NATIVE_UDP_DRIVER_BINDING_ERROR",
            Self::Contract => "AGENT_KERNEL_NATIVE_UDP_DRIVER_CONTRACT_ERROR",
        }
    }
}

pub(super) fn prepare(
    booted: &mut X86BootedKernel,
    device: ResourceCreateOutcome,
    descriptor: DriverEndpointDescriptor,
    contract: BootNetworkDriverImage,
) -> Result<NetworkDriverAdmission, NetworkDriverAdmissionError> {
    let report = *booted.report();
    booted
        .kernel_mut()
        .sys_register_agent(DRIVER)
        .map_err(|_| NetworkDriverAdmissionError::Agent)?;
    let operations = OperationSet::only(Operation::Observe).with(Operation::Act);
    let capability = booted
        .kernel_mut()
        .sys_derive_capability(
            report.bootstrap_agent,
            device.capability,
            DRIVER,
            operations,
        )
        .map_err(|_| NetworkDriverAdmissionError::Capability)?;
    booted
        .kernel_mut()
        .sys_register_driver_endpoint(
            report.bootstrap_agent,
            device.capability,
            device.resource,
            descriptor,
        )
        .map_err(|_| NetworkDriverAdmissionError::Endpoint)?;
    let image = booted
        .kernel_mut()
        .sys_register_agent_image(
            report.bootstrap_agent,
            device.capability,
            device.resource,
            AgentImageKind::Driver,
            contract.digest(),
            1,
            1,
        )
        .map_err(|_| NetworkDriverAdmissionError::ImageRegistration)?;
    booted
        .kernel_mut()
        .sys_verify_agent_image(report.bootstrap_agent, device.capability, image)
        .map_err(|_| NetworkDriverAdmissionError::ImageVerification)?;
    booted
        .kernel_mut()
        .sys_launch_agent(
            DRIVER,
            capability,
            device.resource,
            image,
            AgentEntryKind::Driver,
            None,
        )
        .map_err(|_| NetworkDriverAdmissionError::Launch)?;
    let binding = booted
        .kernel_mut()
        .sys_bind_driver(
            report.bootstrap_agent,
            device.capability,
            device.resource,
            DRIVER,
        )
        .map_err(|_| NetworkDriverAdmissionError::Binding)?;

    let capability_record = booted
        .kernel()
        .capability(capability)
        .map_err(|_| NetworkDriverAdmissionError::Contract)?;
    let endpoint = booted
        .kernel()
        .driver_endpoint(device.resource)
        .map_err(|_| NetworkDriverAdmissionError::Contract)?;
    if capability_record.agent != DRIVER
        || capability_record.resource != device.resource
        || capability_record.operations != operations
        || capability_record.parent != Some(device.capability)
        || endpoint.descriptor != descriptor
    {
        return Err(NetworkDriverAdmissionError::Contract);
    }
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_DRIVER_CAPABILITY_OK");
    Ok(NetworkDriverAdmission {
        capability,
        binding,
        image,
    })
}
